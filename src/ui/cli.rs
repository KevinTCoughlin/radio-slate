use clap::{Parser, ValueEnum};
use serde_json::json;

use crate::application::PlaybackService;
use crate::config::{AppConfig, AppState, FileStore};
use crate::domain::{PlaybackState, Station};
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

pub fn run() {
    let store = FileStore::from_env();
    let config = store.load_config_or_default();
    let state = store.load_state_or_default();
    let args = CliArgs::parse();

    if args.tray {
        if let Err(error) = run_tray(&config) {
            eprintln!("tray mode failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    let startup_station_url = resolve_startup_station_url(&config, &state);
    let seed = vec![build_startup_station(&startup_station_url)];
    let repository = InMemoryStationRepository::with_seed_stations(seed);
    let player = HttpAudioPlayer::new(config.volume_percent);
    let service = PlaybackService::new(repository, player);

    if args.format == OutputFormat::Json {
        let payload = json!({
            "ready": true,
            "stations": service.list_stations(),
            "startup_station_url": startup_station_url,
            "volume_percent": config.volume_percent,
            "tray_icon": config.tray_icon,
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
        if let Err(error) = store.save_state(&AppState {
            last_station_url: Some(station.url),
        }) {
            eprintln!("failed to persist last station: {error}");
        }
    }
}

fn resolve_startup_station_url(config: &AppConfig, state: &AppState) -> String {
    state
        .last_station_url
        .as_ref()
        .filter(|url| is_http_url(url))
        .cloned()
        .or_else(|| {
            if is_http_url(&config.default_station_url) {
                Some(config.default_station_url.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| crate::config::DEFAULT_STATION_URL.to_string())
}

fn build_startup_station(url: &str) -> Station {
    Station::new("Default", url, "mixed").unwrap_or_else(|_| {
        Station::new("KEXP", crate::config::DEFAULT_STATION_URL, "eclectic").unwrap()
    })
}

fn is_http_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::resolve_startup_station_url;
    use crate::config::{AppConfig, AppState, DEFAULT_STATION_URL};

    #[test]
    fn startup_uses_last_station_when_present() {
        let config = AppConfig::default();
        let state = AppState {
            last_station_url: Some("https://example.test/last".to_string()),
        };

        assert_eq!(
            resolve_startup_station_url(&config, &state),
            "https://example.test/last"
        );
    }

    #[test]
    fn startup_uses_config_default_without_last_station() {
        let config = AppConfig {
            default_station_url: "https://example.test/default".to_string(),
            ..AppConfig::default()
        };

        assert_eq!(
            resolve_startup_station_url(&config, &AppState::default()),
            "https://example.test/default"
        );
    }

    #[test]
    fn startup_falls_back_for_invalid_urls() {
        let config = AppConfig {
            default_station_url: "invalid".to_string(),
            ..AppConfig::default()
        };
        let state = AppState {
            last_station_url: Some("still-invalid".to_string()),
        };

        assert_eq!(
            resolve_startup_station_url(&config, &state),
            DEFAULT_STATION_URL
        );
    }
}
