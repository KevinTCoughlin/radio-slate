use radio_slate::application::PlaybackService;
use radio_slate::domain::{PlaybackError, PlaybackState, Station};
use radio_slate::infrastructure::{AudioPlayback, InMemoryStationRepository};

#[derive(Default)]
struct RecordingPlayer {
    played: std::sync::Mutex<usize>,
    stopped: std::sync::Mutex<usize>,
}

impl AudioPlayback for RecordingPlayer {
    fn play_station(&self, _station: &Station) -> Result<(), PlaybackError> {
        *self.played.lock().unwrap() += 1;
        Ok(())
    }

    fn stop_playback(&self) -> Result<(), PlaybackError> {
        *self.stopped.lock().unwrap() += 1;
        Ok(())
    }
}

#[test]
fn playback_service_transitions_through_play_and_stop() {
    let station = Station::new("Echo", "https://example.test/stream", "news").unwrap();
    let repository = InMemoryStationRepository::with_seed_stations(vec![station.clone()]);
    let player = RecordingPlayer::default();
    let mut service = PlaybackService::new(repository, player);

    service.select_station(&station.id).unwrap();
    let playing = service.play_selected().unwrap();
    let stopped = service.stop().unwrap();

    assert!(matches!(playing, PlaybackState::Playing(_)));
    assert_eq!(stopped, PlaybackState::Stopped);
}
