use clap::{Parser, ValueEnum};
use serde_json::json;

use crate::application::PlaybackService;
use crate::domain::{PlaybackState, Station, StationRepository};
use crate::infrastructure::{
    export_to_path, import_from_path, HttpAudioPlayer, JsonStationRepository,
};
use crate::ui::run_tray;

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Parser)]
#[command(name = "radio-slate")]
struct CliArgs {
    #[arg(long, default_value_t = false)]
    play: bool,

    #[arg(long, default_value_t = false)]
    list: bool,

    #[arg(long, default_value_t = false)]
    status: bool,

    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    #[arg(long, default_value_t = false)]
    tray: bool,

    /// Add a station to the library by stream URL.
    /// Use --name and --genre to set metadata (both default to a value
    /// derived from the URL / "imported" if omitted).
    #[arg(long, value_name = "URL")]
    add_station: Option<String>,

    /// Name for the station added with --add-station.
    #[arg(long, requires = "add_station")]
    name: Option<String>,

    /// Genre for the station added with --add-station.
    #[arg(long, requires = "add_station")]
    genre: Option<String>,

    /// Import stations from a PLS, M3U/M3U8, or JSON file.
    #[arg(long, value_name = "FILE")]
    import: Option<std::path::PathBuf>,

    /// Export all stations to a file (.json, .m3u, or .m3u8).
    #[arg(long, value_name = "FILE")]
    export: Option<std::path::PathBuf>,
}

pub fn run() {
    let args = CliArgs::parse();

    if args.tray {
        if let Err(error) = run_tray() {
            eprintln!("tray mode failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    // Resolve the persistent station store path.
    let store_path = match JsonStationRepository::default_path() {
        Ok(p) => p,
        Err(error) => {
            eprintln!("could not determine station library path: {error}");
            std::process::exit(1);
        }
    };

    let mut repository = match JsonStationRepository::open(&store_path) {
        Ok(r) => r,
        Err(error) => {
            eprintln!("failed to open station library: {error}");
            std::process::exit(1);
        }
    };

    // --add-station
    if let Some(url) = &args.add_station {
        let name = args.name.clone().unwrap_or_else(|| {
            url.split('/').next_back().unwrap_or(url.as_str()).to_string()
        });
        let genre = args.genre.clone().unwrap_or_else(|| "imported".to_string());
        match Station::new(&name, url, &genre) {
            Ok(station) => {
                if let Err(error) = repository.add(station) {
                    eprintln!("failed to add station: {error}");
                    std::process::exit(1);
                }
                println!("added station '{name}' to library");
            }
            Err(error) => {
                eprintln!("invalid station: {error}");
                std::process::exit(1);
            }
        }
    }

    // --import
    if let Some(import_path) = &args.import {
        match import_from_path(import_path) {
            Ok(stations) => {
                let n = stations.len();
                match repository.add_many(stations) {
                    Ok(added) => println!("imported {added} of {n} stations (duplicates skipped)"),
                    Err(error) => {
                        eprintln!("import failed: {error}");
                        std::process::exit(1);
                    }
                }
            }
            Err(error) => {
                eprintln!("failed to read import file: {error}");
                std::process::exit(1);
            }
        }
    }

    // --export
    if let Some(export_path) = &args.export {
        let stations = repository.list();
        match export_to_path(&stations, export_path) {
            Ok(()) => println!("exported {} stations to '{}'", stations.len(), export_path.display()),
            Err(error) => {
                eprintln!("export failed: {error}");
                std::process::exit(1);
            }
        }
    }

    let player = HttpAudioPlayer::new();
    let service = PlaybackService::new(repository, player);

    if args.format == OutputFormat::Json {
        let payload = json!({
            "ready": true,
            "stations": service.list_stations(),
            "default_playback_state": service.status_label(PlaybackState::Stopped),
            "play_requested": args.play,
            "list_requested": args.list,
            "status_requested": args.status,
        });
        println!("{payload}");
    } else {
        println!("radio-slate ready");
        println!("available stations: {}", service.list_stations().len());
        println!(
            "default playback state: {}",
            service.status_label(PlaybackState::Stopped)
        );
    }

    if args.list {
        let stations = service.list_stations();
        if args.format == OutputFormat::Json {
            println!("{}", serde_json::to_string_pretty(&stations).unwrap());
        } else {
            for station in stations {
                println!("{} | {} | {}", station.name, station.genre, station.url);
            }
        }
    }

    if args.status {
        if args.format == OutputFormat::Json {
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &json!({"status": service.status_label(PlaybackState::Stopped)})
                )
                .unwrap()
            );
        } else {
            println!("status: {}", service.status_label(PlaybackState::Stopped));
        }
    }

    if args.play {
        let station = service.list_stations().into_iter().next().unwrap();
        if let Err(error) = service.play_station(&station) {
            eprintln!("playback failed: {error}");
            std::process::exit(1);
        }
    }
}
