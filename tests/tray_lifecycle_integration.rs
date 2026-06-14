use radio_slate::application::PlaybackService;
use radio_slate::domain::{PlaybackError, PlaybackState, Station};
use radio_slate::infrastructure::{AudioPlayback, InMemoryStationRepository};
use radio_slate::ui::tray::{toggle_tray_playback, tray_toggle_label};

#[derive(Default)]
struct StubPlayer;

impl AudioPlayback for StubPlayer {
    fn play_station(&self, _station: &Station) -> Result<(), PlaybackError> {
        Ok(())
    }

    fn stop_playback(&self) -> Result<(), PlaybackError> {
        Ok(())
    }
}

#[test]
fn tray_toggle_uses_shared_playback_service_lifecycle() {
    let station = Station::new("Echo", "https://example.test/stream", "news").unwrap();
    let repository = InMemoryStationRepository::with_seed_stations(vec![station]);
    let mut service = PlaybackService::new(repository, StubPlayer);

    let state = toggle_tray_playback(&mut service).unwrap();
    assert_eq!(tray_toggle_label(&state), "Stop KEXP");
    assert!(matches!(state, PlaybackState::Playing(_)));

    let state = toggle_tray_playback(&mut service).unwrap();
    assert_eq!(tray_toggle_label(&state), "Play KEXP");
    assert_eq!(state, PlaybackState::Stopped);
}
