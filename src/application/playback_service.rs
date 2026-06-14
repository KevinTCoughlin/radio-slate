use crate::domain::{PlaybackState, Station, StationId, StationRepository};
use crate::infrastructure::AudioPlayback;

pub struct PlaybackService<R: StationRepository, P: AudioPlayback> {
    repository: R,
    player: P,
}

impl<R: StationRepository, P: AudioPlayback> PlaybackService<R, P> {
    pub fn new(repository: R, player: P) -> Self {
        Self { repository, player }
    }

    pub fn list_stations(&self) -> Vec<Station> {
        self.repository.list()
    }

    pub fn preview_station(&self, id: &StationId) -> Option<Station> {
        self.repository.get(id)
    }

    pub fn status_label(&self, state: PlaybackState) -> &'static str {
        self.player.status_label(state)
    }

    pub fn play_station(&self, station: &Station) -> anyhow::Result<()> {
        self.player.play_station(station)
    }
}

#[cfg(test)]
mod tests {
    use super::PlaybackService;
    use crate::domain::{PlaybackState, Station, StationId, StationRepository};
    use crate::infrastructure::AudioPlayback;

    #[derive(Default)]
    struct StubRepo {
        stations: Vec<Station>,
    }

    impl StationRepository for StubRepo {
        fn list(&self) -> Vec<Station> {
            self.stations.clone()
        }

        fn get(&self, id: &StationId) -> Option<Station> {
            self.stations
                .iter()
                .find(|station| station.id == *id)
                .cloned()
        }
    }

    #[derive(Default)]
    struct StubPlayer;

    impl AudioPlayback for StubPlayer {
        fn play_station(&self, _station: &Station) -> anyhow::Result<()> {
            Ok(())
        }

        fn status_label(&self, state: PlaybackState) -> &'static str {
            match state {
                PlaybackState::Stopped => "stopped",
                PlaybackState::Playing => "playing",
                PlaybackState::Buffering => "buffering",
            }
        }
    }

    #[test]
    fn playback_service_lists_available_stations() {
        let station = Station::new("Echo", "https://example.test/stream", "news").unwrap();
        let repo = StubRepo {
            stations: vec![station.clone()],
        };
        let service = PlaybackService::new(repo, StubPlayer);

        assert_eq!(service.list_stations().len(), 1);
        assert_eq!(service.preview_station(&station.id), Some(station));
    }

    #[test]
    fn playback_service_describes_state() {
        let service = PlaybackService::new(StubRepo::default(), StubPlayer);

        assert_eq!(service.status_label(PlaybackState::Playing), "playing");
    }
}
