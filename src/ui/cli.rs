use clap::{Parser, ValueEnum};
use serde_json::json;

use crate::application::PlaybackService;
use crate::domain::Station;
use crate::infrastructure::{HttpAudioPlayer, InMemoryStationRepository};
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
}

const DEFAULT_STATION_URL: &str = "http://live-mp3-128.kexp.org/kexp128.mp3";

pub fn run() {
    let args = CliArgs::parse();

    if args.tray {
        if let Err(error) = run_tray() {
            eprintln!("tray mode failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    let seed = vec![Station::new("KEXP", DEFAULT_STATION_URL, "eclectic").unwrap()];
    let repository = InMemoryStationRepository::with_seed_stations(seed);
    let player = HttpAudioPlayer::new();
    let mut service = PlaybackService::new(repository, player);
    let default_selection = service.select_default_station().ok();

    if args.format == OutputFormat::Json {
        let payload = json!({
            "ready": true,
            "stations": service.list_stations(),
            "default_station_selected": default_selection.is_some(),
            "default_playback_state": service.status_label(),
            "play_requested": args.play,
            "list_requested": args.list,
            "status_requested": args.status,
        });
        println!("{payload}");
    } else {
        println!("radio-slate ready");
        println!("available stations: {}", service.list_stations().len());
        println!("default playback state: {}", service.status_label());
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
                serde_json::to_string_pretty(&json!({"status": service.status_label()})).unwrap()
            );
        } else {
            println!("status: {}", service.status_label());
        }
    }

    if args.play {
        if default_selection.is_none() {
            eprintln!("playback failed: no stations configured");
            std::process::exit(1);
        }
        if let Err(error) = service.play_selected() {
            eprintln!("playback failed: {error}");
            std::process::exit(1);
        }
    }
}
