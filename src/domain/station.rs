use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Station {
    pub id: StationId,
    pub name: String,
    pub url: String,
    pub genre: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StationId(String);

impl StationId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl AsRef<str> for StationId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for StationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationSelection {
    station_id: StationId,
}

impl StationSelection {
    pub fn new(station_id: StationId) -> Self {
        Self { station_id }
    }

    pub fn station_id(&self) -> &StationId {
        &self.station_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Buffering(StationSelection),
    Playing(StationSelection),
}

impl PlaybackState {
    pub fn status_label(&self) -> &'static str {
        match self {
            PlaybackState::Stopped => "stopped",
            PlaybackState::Playing(_) => "playing",
            PlaybackState::Buffering(_) => "buffering",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackError {
    NoStationsConfigured,
    StationNotFound(StationId),
    PlayerUnavailable(String),
}

impl std::fmt::Display for PlaybackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlaybackError::NoStationsConfigured => write!(f, "no stations configured"),
            PlaybackError::StationNotFound(id) => {
                write!(f, "station not found: {}", id.as_ref())
            }
            PlaybackError::PlayerUnavailable(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for PlaybackError {}

impl From<std::io::Error> for PlaybackError {
    fn from(value: std::io::Error) -> Self {
        Self::PlayerUnavailable(format!("{value}"))
    }
}

pub trait StationRepository {
    fn list(&self) -> Vec<Station>;
    fn get(&self, id: &StationId) -> Option<Station>;
}

/// Extension of [`StationRepository`] that supports mutating the station library.
///
/// Both durable backends ([`JsonStationRepository`] and [`SqliteStationRepository`])
/// implement this trait, which makes swapping backends a matter of choosing a
/// concrete type at startup.
///
/// [`JsonStationRepository`]: crate::infrastructure::JsonStationRepository
/// [`SqliteStationRepository`]: crate::infrastructure::SqliteStationRepository
pub trait MutableStationRepository: StationRepository {
    /// Add a station, returning an error if a station with the same id already exists.
    fn add(&mut self, station: Station) -> anyhow::Result<()>;
    /// Remove the station with `id`.  Returns `true` if a station was removed.
    fn remove(&mut self, id: &StationId) -> anyhow::Result<bool>;
    /// Add multiple stations, silently skipping duplicates.
    /// Returns the number actually inserted.
    fn add_many(&mut self, stations: Vec<Station>) -> anyhow::Result<usize>;
}

impl StationRepository for Box<dyn MutableStationRepository> {
    fn list(&self) -> Vec<Station> {
        (**self).list()
    }

    fn get(&self, id: &StationId) -> Option<Station> {
        (**self).get(id)
    }
}

impl Station {
    pub fn new(name: &str, url: &str, genre: &str) -> Result<Self, String> {
        let name = name.trim().to_string();
        let url = url.trim().to_string();
        let genre = genre.trim().to_string();

        if name.is_empty() || url.is_empty() || genre.is_empty() {
            return Err("station name, url, and genre must all be set".to_string());
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err("station url must use http or https".to_string());
        }

        Ok(Self {
            id: StationId::new(format!(
                "station-{}",
                name.to_ascii_lowercase().replace(' ', "-")
            )),
            name,
            url,
            genre,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{PlaybackState, Station, StationId, StationSelection};

    #[test]
    fn station_rejects_invalid_input() {
        assert!(Station::new("", "https://example.test/stream", "music").is_err());
    }

    #[test]
    fn station_accepts_valid_http_stream() {
        let station = Station::new("Echo", "https://example.test/stream", "news").unwrap();
        assert_eq!(station.genre, "news");
        assert_eq!(station.url, "https://example.test/stream");
    }

    #[test]
    fn station_id_display_matches_inner_value() {
        let id = StationId::new("station-echo");
        assert_eq!(id.to_string(), "station-echo");
    }

    #[test]
    fn station_id_derived_from_name() {
        let station = Station::new("My Jazz", "https://example.test/stream", "jazz").unwrap();
        assert_eq!(station.id.to_string(), "station-my-jazz");
    }

    #[test]
    fn station_rejects_non_http_url() {
        assert!(Station::new("Bad", "ftp://example.test/stream", "rock").is_err());
    }

    #[test]
    fn playback_state_label_matches_variant() {
        let selection = StationSelection::new(StationId::new("station-echo"));
        assert_eq!(PlaybackState::Stopped.status_label(), "stopped");
        assert_eq!(
            PlaybackState::Buffering(selection.clone()).status_label(),
            "buffering"
        );
        assert_eq!(PlaybackState::Playing(selection).status_label(), "playing");
    }
}
