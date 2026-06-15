use crate::application::PlaybackSnapshot;
use crate::domain::{PlaybackState, Station, StationId, StationRepository};
use crate::infrastructure::{AudioPlayback, OutputSink};

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

    /// Emit a [`PlaybackSnapshot`] for the given `state` to `sink`.
    ///
    /// The snapshot is serialized to a single JSON line.  Callers supply the
    /// concrete sink (stdout, file, or in-memory buffer) so the service stays
    /// decoupled from any specific output destination.
    pub fn emit_snapshot(
        &self,
        state: PlaybackState,
        station: Option<Station>,
        sink: &mut dyn OutputSink,
    ) -> anyhow::Result<()> {
        let snapshot = PlaybackSnapshot {
            state: self.status_label(state).to_string(),
            station,
            stations_available: self.list_stations().len(),
        };
        let json = snapshot.to_json()?;
        sink.emit(&json)?;
        sink.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::PlaybackService;
    use crate::domain::{PlaybackState, Station, StationId, StationRepository};
    use crate::infrastructure::{AudioPlayback, BufferedSink};

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

    #[test]
    fn emit_snapshot_writes_json_to_sink() {
        let station = Station::new("Echo", "https://example.test/stream", "news").unwrap();
        let repo = StubRepo {
            stations: vec![station.clone()],
        };
        let service = PlaybackService::new(repo, StubPlayer);
        let mut sink = BufferedSink::new();

        service
            .emit_snapshot(PlaybackState::Stopped, None, &mut sink)
            .unwrap();

        assert_eq!(sink.lines.len(), 1);
        let line = &sink.lines[0];
        assert!(line.contains("\"state\""));
        assert!(line.contains("stopped"));
        assert!(line.contains("stations_available"));
    }

    #[test]
    fn emit_snapshot_includes_station_count() {
        let stations = vec![
            Station::new("A", "https://a.test/stream", "rock").unwrap(),
            Station::new("B", "https://b.test/stream", "jazz").unwrap(),
        ];
        let repo = StubRepo { stations };
        let service = PlaybackService::new(repo, StubPlayer);
        let mut sink = BufferedSink::new();

        service
            .emit_snapshot(PlaybackState::Playing, None, &mut sink)
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&sink.lines[0]).unwrap();
        assert_eq!(parsed["stations_available"], 2);
        assert_eq!(parsed["state"], "playing");
    }
}
