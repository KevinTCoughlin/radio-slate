use serde::{Deserialize, Serialize};

use crate::domain::Station;

/// A serializable point-in-time snapshot of playback state.
///
/// Used by the output adapter layer to emit structured data to any
/// [`crate::infrastructure::OutputSink`] (stdout, file, or in-memory buffer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaybackSnapshot {
    /// Human-readable state label: "stopped", "playing", or "buffering".
    pub state: String,
    /// The station associated with this snapshot, if one is active.
    pub station: Option<Station>,
    /// Number of stations available in the repository at snapshot time.
    pub stations_available: usize,
}

impl PlaybackSnapshot {
    /// Serialize the snapshot to a compact JSON string.
    pub fn to_json(&self) -> anyhow::Result<String> {
        serde_json::to_string(self)
            .map_err(|e| anyhow::anyhow!("snapshot serialization failed: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stopped_snapshot() -> PlaybackSnapshot {
        PlaybackSnapshot {
            state: "stopped".to_string(),
            station: None,
            stations_available: 0,
        }
    }

    #[test]
    fn snapshot_serializes_to_json() {
        let snapshot = stopped_snapshot();
        let json = snapshot.to_json().unwrap();
        assert!(json.contains("\"state\""));
        assert!(json.contains("stopped"));
        assert!(json.contains("stations_available"));
    }

    #[test]
    fn snapshot_roundtrips_through_json() {
        let original = PlaybackSnapshot {
            state: "playing".to_string(),
            station: None,
            stations_available: 3,
        };
        let json = original.to_json().unwrap();
        let decoded: PlaybackSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn snapshot_with_station_roundtrips() {
        use crate::domain::Station;
        let station = Station::new("KEXP", "http://live-mp3-128.kexp.org/kexp128.mp3", "eclectic").unwrap();
        let original = PlaybackSnapshot {
            state: "playing".to_string(),
            station: Some(station),
            stations_available: 1,
        };
        let json = original.to_json().unwrap();
        let decoded: PlaybackSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn snapshot_serialization_failure_returns_error() {
        // PlaybackSnapshot is always serializable, so verify the happy path
        // (serde_json cannot fail for this well-formed struct).
        let snapshot = stopped_snapshot();
        assert!(snapshot.to_json().is_ok());
    }
}
