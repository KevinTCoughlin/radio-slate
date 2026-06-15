use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::domain::{MutableStationRepository, Station, StationId, StationRepository};

const DEFAULT_STATION_NAME: &str = "KEXP";
const DEFAULT_STATION_URL: &str = "http://live-mp3-128.kexp.org/kexp128.mp3";
const DEFAULT_STATION_GENRE: &str = "eclectic";
const STORE_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct StoreData {
    version: u32,
    stations: Vec<Station>,
}

/// A durable station repository that persists to a JSON file on disk.
///
/// On first creation (no file present) the KEXP stream is seeded as a
/// built-in fallback so the app is usable out of the box.  A `version`
/// field in the envelope allows forward-compatible migrations.
pub struct JsonStationRepository {
    path: PathBuf,
    stations: Vec<Station>,
}

impl JsonStationRepository {
    /// Open (or create) the store at `path`.
    ///
    /// * If the file does not exist the repository is seeded with the KEXP
    ///   built-in station and the file is **not** written yet (it is created
    ///   lazily on the first mutating call or an explicit [`save`]).
    /// * If the file exists in the legacy plain-array format (v0) it is
    ///   accepted as-is; the versioned envelope will be written on the next
    ///   [`save`] call, completing the migration.
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let stations = if path.exists() {
            let content = fs::read_to_string(&path)?;
            // Try the versioned envelope first, then fall back to the legacy
            // plain-array format (v0 → v1 migration).
            if let Ok(data) = serde_json::from_str::<StoreData>(&content) {
                data.stations
            } else if let Ok(legacy) = serde_json::from_str::<Vec<Station>>(&content) {
                legacy
            } else {
                anyhow::bail!(
                    "station store at '{}' has an unrecognised format",
                    path.display()
                );
            }
        } else {
            // First-time initialisation: seed with the KEXP built-in fallback.
            let kexp = Station::new(
                DEFAULT_STATION_NAME,
                DEFAULT_STATION_URL,
                DEFAULT_STATION_GENRE,
            )
            .map_err(|e| anyhow::anyhow!(e))?;
            vec![kexp]
        };

        Ok(Self { path, stations })
    }

    /// Return the platform-appropriate default path for the station store:
    /// `$XDG_CONFIG_HOME/radio-slate/stations.json` or
    /// `~/.config/radio-slate/stations.json`.
    pub fn default_path() -> anyhow::Result<PathBuf> {
        let config_root = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg)
        } else {
            let home = std::env::var("HOME")
                .map_err(|_| anyhow::anyhow!("HOME environment variable is not set"))?;
            PathBuf::from(home).join(".config")
        };
        Ok(config_root.join("radio-slate").join("stations.json"))
    }

    /// Persist the current state to disk, creating parent directories as needed.
    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = StoreData {
            version: STORE_VERSION,
            stations: self.stations.clone(),
        };
        let content = serde_json::to_string_pretty(&data)?;
        fs::write(&self.path, content)?;
        Ok(())
    }

    /// Add a station to the repository and persist immediately.
    ///
    /// Returns an error if a station with the same `id` already exists.
    pub fn add(&mut self, station: Station) -> anyhow::Result<()> {
        if self.stations.iter().any(|s| s.id == station.id) {
            anyhow::bail!(
                "station '{}' already exists in the library",
                station.id.as_ref()
            );
        }
        self.stations.push(station);
        self.save()
    }

    /// Remove the station with the given `id` and persist immediately.
    ///
    /// Returns `true` if a station was removed, `false` if nothing matched.
    pub fn remove(&mut self, id: &StationId) -> anyhow::Result<bool> {
        let before = self.stations.len();
        self.stations.retain(|s| s.id != *id);
        let removed = self.stations.len() < before;
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    /// Add multiple stations at once and persist once at the end.
    ///
    /// Stations whose `id` already exists in the library are silently skipped.
    /// Returns the number of stations actually added.
    pub fn add_many(&mut self, stations: Vec<Station>) -> anyhow::Result<usize> {
        let mut added = 0usize;
        for station in stations {
            if !self.stations.iter().any(|s| s.id == station.id) {
                self.stations.push(station);
                added += 1;
            }
        }
        if added > 0 {
            self.save()?;
        }
        Ok(added)
    }
}

impl StationRepository for JsonStationRepository {
    fn list(&self) -> Vec<Station> {
        self.stations.clone()
    }

    fn get(&self, id: &StationId) -> Option<Station> {
        self.stations.iter().find(|s| s.id == *id).cloned()
    }
}

impl MutableStationRepository for JsonStationRepository {
    fn add(&mut self, station: Station) -> anyhow::Result<()> {
        JsonStationRepository::add(self, station)
    }

    fn remove(&mut self, id: &StationId) -> anyhow::Result<bool> {
        JsonStationRepository::remove(self, id)
    }

    fn add_many(&mut self, stations: Vec<Station>) -> anyhow::Result<usize> {
        JsonStationRepository::add_many(self, stations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn tmp_path(dir: &TempDir) -> PathBuf {
        dir.path().join("stations.json")
    }

    #[test]
    fn new_store_seeds_kexp_fallback_when_file_absent() {
        let dir = TempDir::new().unwrap();
        let repo = JsonStationRepository::open(tmp_path(&dir)).unwrap();
        let stations = repo.list();
        assert_eq!(stations.len(), 1);
        assert_eq!(stations[0].name, DEFAULT_STATION_NAME);
        assert_eq!(stations[0].url, DEFAULT_STATION_URL);
    }

    #[test]
    fn save_and_reload_round_trips_stations() {
        let dir = TempDir::new().unwrap();
        let path = tmp_path(&dir);

        let station = Station::new("Test FM", "https://example.test/stream", "pop").unwrap();
        let mut repo = JsonStationRepository::open(&path).unwrap();
        repo.add(station.clone()).unwrap();

        let reloaded = JsonStationRepository::open(&path).unwrap();
        assert!(reloaded.list().iter().any(|s| s.id == station.id));
    }

    #[test]
    fn add_rejects_duplicate_station() {
        let dir = TempDir::new().unwrap();
        let mut repo = JsonStationRepository::open(tmp_path(&dir)).unwrap();
        let station = Station::new("Echo", "https://example.test/stream", "jazz").unwrap();
        repo.add(station.clone()).unwrap();
        let result = repo.add(station);
        assert!(result.is_err());
    }

    #[test]
    fn remove_deletes_station_and_persists() {
        let dir = TempDir::new().unwrap();
        let path = tmp_path(&dir);
        let station = Station::new("Echo", "https://example.test/stream", "jazz").unwrap();
        let id = station.id.clone();

        let mut repo = JsonStationRepository::open(&path).unwrap();
        repo.add(station).unwrap();
        let removed = repo.remove(&id).unwrap();
        assert!(removed);
        assert!(repo.get(&id).is_none());

        // Verify the deletion survived a reload.
        let reloaded = JsonStationRepository::open(&path).unwrap();
        assert!(reloaded.get(&id).is_none());
    }

    #[test]
    fn remove_returns_false_for_unknown_id() {
        let dir = TempDir::new().unwrap();
        let mut repo = JsonStationRepository::open(tmp_path(&dir)).unwrap();
        let id = StationId::new("station-nonexistent");
        assert!(!repo.remove(&id).unwrap());
    }

    #[test]
    fn migrates_legacy_plain_array_format() {
        let dir = TempDir::new().unwrap();
        let path = tmp_path(&dir);

        // Write a v0 plain-array file.
        let legacy_station =
            Station::new("Legacy FM", "https://example.test/legacy", "oldies").unwrap();
        let v0_json = serde_json::to_string(&vec![legacy_station.clone()]).unwrap();
        fs::write(&path, v0_json).unwrap();

        let repo = JsonStationRepository::open(&path).unwrap();
        assert_eq!(repo.list().len(), 1);
        assert_eq!(repo.list()[0].name, "Legacy FM");
    }

    #[test]
    fn add_many_skips_duplicates_and_returns_count() {
        let dir = TempDir::new().unwrap();
        let mut repo = JsonStationRepository::open(tmp_path(&dir)).unwrap();
        // The repo already has KEXP seeded.
        let kexp = repo.list()[0].clone();
        let new_station =
            Station::new("Fresh Radio", "https://example.test/fresh", "indie").unwrap();
        let count = repo.add_many(vec![kexp, new_station]).unwrap();
        assert_eq!(count, 1); // only the new one was inserted
    }
}
