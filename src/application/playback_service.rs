use crate::application::PlaybackSnapshot;
use crate::domain::{
    PlaybackError, PlaybackState, Station, StationId, StationRepository, StationSelection,
};
use crate::infrastructure::{AudioPlayback, OutputSink};

pub struct PlaybackService<R: StationRepository, P: AudioPlayback> {
    repository: R,
    player: P,
    selected_station: Option<StationSelection>,
    playback_state: PlaybackState,
}

impl<R: StationRepository, P: AudioPlayback> PlaybackService<R, P> {
    pub fn new(repository: R, player: P) -> Self {
        Self {
            repository,
            player,
            selected_station: None,
            playback_state: PlaybackState::Stopped,
        }
    }

    pub fn list_stations(&self) -> Vec<Station> {
        self.repository.list()
    }

    pub fn preview_station(&self, id: &StationId) -> Option<Station> {
        self.repository.get(id)
    }

    pub fn select_station(&mut self, id: &StationId) -> Result<StationSelection, PlaybackError> {
        if self.repository.get(id).is_none() {
            return Err(PlaybackError::StationNotFound(id.clone()));
        }

        let selection = StationSelection::new(id.clone());
        self.selected_station = Some(selection.clone());
        Ok(selection)
    }

    pub fn select_default_station(&mut self) -> Result<StationSelection, PlaybackError> {
        let station = self
            .list_stations()
            .into_iter()
            .next()
            .ok_or(PlaybackError::NoStationsConfigured)?;
        self.select_station(&station.id)
    }

    pub fn selected_station(&self) -> Option<StationSelection> {
        self.selected_station.clone()
    }

    pub fn playback_state(&self) -> PlaybackState {
        self.playback_state.clone()
    }

    pub fn status_label(&self) -> &'static str {
        self.playback_state.status_label()
    }

    pub fn play_selected(&mut self) -> Result<PlaybackState, PlaybackError> {
        let selection = self
            .selected_station
            .clone()
            .ok_or(PlaybackError::NoStationsConfigured)?;
        let station = self
            .repository
            .get(selection.station_id())
            .ok_or_else(|| PlaybackError::StationNotFound(selection.station_id().clone()))?;

        self.playback_state = PlaybackState::Buffering(selection.clone());
        self.player.play_station(&station)?;
        self.playback_state = PlaybackState::Playing(selection.clone());
        Ok(self.playback_state.clone())
    }

    pub fn stop(&mut self) -> Result<PlaybackState, PlaybackError> {
        self.player.stop_playback()?;
        self.playback_state = PlaybackState::Stopped;
        Ok(self.playback_state.clone())
    }

    pub fn toggle_selected(&mut self) -> Result<PlaybackState, PlaybackError> {
        match self.playback_state {
            PlaybackState::Stopped => self.play_selected(),
            PlaybackState::Buffering(_) | PlaybackState::Playing(_) => self.stop(),
        }
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
            state: state.status_label().to_string(),
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
    use crate::domain::{
        PlaybackError, PlaybackState, Station, StationId, StationRepository, StationSelection,
    };
    use crate::infrastructure::{AudioPlayback, BufferedSink};
    use std::sync::Mutex;

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
    struct StubPlayer {
        plays: Mutex<usize>,
        stops: Mutex<usize>,
    }

    impl AudioPlayback for StubPlayer {
        fn play_station(&self, _station: &Station) -> Result<(), PlaybackError> {
            *self.plays.lock().unwrap() += 1;
            Ok(())
        }

        fn stop_playback(&self) -> Result<(), PlaybackError> {
            *self.stops.lock().unwrap() += 1;
            Ok(())
        }
    }

    #[test]
    fn playback_service_lists_available_stations() {
        let station = Station::new("Echo", "https://example.test/stream", "news").unwrap();
        let repo = StubRepo {
            stations: vec![station.clone()],
        };
        let service = PlaybackService::new(repo, StubPlayer::default());

        assert_eq!(service.list_stations().len(), 1);
        assert_eq!(service.preview_station(&station.id), Some(station));
    }

    #[test]
    fn playback_service_tracks_selection_and_state() {
        let station = Station::new("Echo", "https://example.test/stream", "news").unwrap();
        let repo = StubRepo {
            stations: vec![station.clone()],
        };
        let mut service = PlaybackService::new(repo, StubPlayer::default());

        let selection = service.select_station(&station.id).unwrap();
        let state = service.play_selected().unwrap();

        assert_eq!(selection.station_id(), &station.id);
        assert_eq!(state, PlaybackState::Playing(selection));
        assert_eq!(service.status_label(), "playing");
    }

    #[test]
    fn playback_service_toggle_stops_playback() {
        let station = Station::new("Echo", "https://example.test/stream", "news").unwrap();
        let repo = StubRepo {
            stations: vec![station.clone()],
        };
        let mut service = PlaybackService::new(repo, StubPlayer::default());

        service.select_station(&station.id).unwrap();
        service.play_selected().unwrap();
        let state = service.toggle_selected().unwrap();

        assert_eq!(state, PlaybackState::Stopped);
    }

    #[test]
    fn playback_service_reports_missing_station_errors() {
        let mut service = PlaybackService::new(StubRepo::default(), StubPlayer::default());
        let err = service.select_default_station().unwrap_err();

        assert_eq!(err, PlaybackError::NoStationsConfigured);
    }

    #[test]
    fn emit_snapshot_writes_json_to_sink() {
        let station = Station::new("Echo", "https://example.test/stream", "news").unwrap();
        let repo = StubRepo {
            stations: vec![station.clone()],
        };
        let service = PlaybackService::new(repo, StubPlayer::default());
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
        let repo = StubRepo {
            stations: stations.clone(),
        };
        let service = PlaybackService::new(repo, StubPlayer::default());
        let mut sink = BufferedSink::new();

        service
            .emit_snapshot(
                PlaybackState::Playing(StationSelection::new(stations[0].id.clone())),
                None,
                &mut sink,
            )
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&sink.lines[0]).unwrap();
        assert_eq!(parsed["stations_available"], 2);
        assert_eq!(parsed["state"], "playing");
    }
}
