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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Buffering,
}

pub trait StationRepository {
    fn list(&self) -> Vec<Station>;
    fn get(&self, id: &StationId) -> Option<Station>;
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
    use super::{Station, StationId};

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
}
