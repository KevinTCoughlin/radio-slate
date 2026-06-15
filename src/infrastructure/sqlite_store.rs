use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

use crate::domain::{Station, StationId, StationRepository};

const DEFAULT_STATION_NAME: &str = "KEXP";
const DEFAULT_STATION_URL: &str = "http://live-mp3-128.kexp.org/kexp128.mp3";
const DEFAULT_STATION_GENRE: &str = "eclectic";

/// A durable station repository backed by an SQLite database.
///
/// The database is created automatically on first open and the schema is
/// kept up-to-date through an internal migration table.  A default KEXP
/// station is seeded when the database is empty so the app works out of
/// the box.
///
/// The file lives at `$XDG_DATA_HOME/radio-slate/stations.db`
/// (falling back to `~/.local/share/radio-slate/stations.db`).
pub struct SqliteStationRepository {
    conn: Connection,
}

impl SqliteStationRepository {
    /// Open (or create) the SQLite database at `path`.
    ///
    /// Runs any pending schema migrations and seeds the default KEXP station
    /// when no stations are present.
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let mut repo = Self { conn };
        repo.migrate()?;
        repo.seed_default_if_empty()?;
        Ok(repo)
    }

    /// Return the platform-appropriate default path for the database:
    /// `$XDG_DATA_HOME/radio-slate/stations.db` or
    /// `~/.local/share/radio-slate/stations.db`.
    pub fn default_path() -> anyhow::Result<PathBuf> {
        let data_root = if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            PathBuf::from(xdg)
        } else {
            let home = std::env::var("HOME")
                .map_err(|_| anyhow::anyhow!("HOME environment variable is not set"))?;
            PathBuf::from(home).join(".local").join("share")
        };
        Ok(data_root.join("radio-slate").join("stations.db"))
    }

    // ── Schema migrations ─────────────────────────────────────────────────────

    fn migrate(&self) -> anyhow::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version  INTEGER PRIMARY KEY
            );

            CREATE TABLE IF NOT EXISTS stations (
                id       TEXT    PRIMARY KEY,
                name     TEXT    NOT NULL,
                url      TEXT    NOT NULL,
                genre    TEXT    NOT NULL
            );",
        )?;

        let applied: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|n| n > 0)
            .unwrap_or(false);

        if !applied {
            // Migration v1 — initial schema (tables created above).
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version) VALUES (1)",
                [],
            )?;
        }

        Ok(())
    }

    fn seed_default_if_empty(&mut self) -> anyhow::Result<()> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM stations", [], |row| row.get(0))?;
        if count == 0 {
            let kexp = Station::new(DEFAULT_STATION_NAME, DEFAULT_STATION_URL, DEFAULT_STATION_GENRE)
                .map_err(|e| anyhow::anyhow!(e))?;
            self.insert_station(&kexp)?;
        }
        Ok(())
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn insert_station(&self, station: &Station) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO stations (id, name, url, genre) VALUES (?1, ?2, ?3, ?4)",
            params![
                station.id.as_ref(),
                station.name,
                station.url,
                station.genre
            ],
        )?;
        Ok(())
    }

    fn row_to_station(row: &rusqlite::Row<'_>) -> rusqlite::Result<Station> {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let url: String = row.get(2)?;
        let genre: String = row.get(3)?;
        Ok(Station {
            id: StationId::new(id),
            name,
            url,
            genre,
        })
    }
}

impl StationRepository for SqliteStationRepository {
    fn list(&self) -> Vec<Station> {
        let mut stmt = match self
            .conn
            .prepare("SELECT id, name, url, genre FROM stations ORDER BY name")
        {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map([], Self::row_to_station)
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
    }

    fn get(&self, id: &StationId) -> Option<Station> {
        self.conn
            .query_row(
                "SELECT id, name, url, genre FROM stations WHERE id = ?1",
                params![id.as_ref()],
                Self::row_to_station,
            )
            .ok()
    }

    fn add(&mut self, station: Station) -> Result<(), String> {
        if self.get(&station.id).is_some() {
            return Err(format!(
                "station '{}' already exists in the library",
                station.id.as_ref()
            ));
        }
        self.insert_station(&station).map_err(|e| e.to_string())
    }

    fn remove(&mut self, id: &StationId) -> Result<bool, String> {
        let rows = self
            .conn
            .execute(
                "DELETE FROM stations WHERE id = ?1",
                params![id.as_ref()],
            )
            .map_err(|e| e.to_string())?;
        Ok(rows > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp_path(dir: &TempDir) -> PathBuf {
        dir.path().join("stations.db")
    }

    fn open(dir: &TempDir) -> SqliteStationRepository {
        SqliteStationRepository::open(tmp_path(dir)).unwrap()
    }

    #[test]
    fn new_database_seeds_kexp_fallback() {
        let dir = TempDir::new().unwrap();
        let repo = open(&dir);
        let stations = repo.list();
        assert_eq!(stations.len(), 1);
        assert_eq!(stations[0].name, DEFAULT_STATION_NAME);
        assert_eq!(stations[0].url, DEFAULT_STATION_URL);
    }

    #[test]
    fn add_and_list_round_trips() {
        let dir = TempDir::new().unwrap();
        let mut repo = open(&dir);
        let station = Station::new("Echo FM", "https://example.test/stream", "jazz").unwrap();
        repo.add(station.clone()).unwrap();
        assert!(repo.list().iter().any(|s| s.id == station.id));
    }

    #[test]
    fn add_rejects_duplicate() {
        let dir = TempDir::new().unwrap();
        let mut repo = open(&dir);
        let station = Station::new("Echo FM", "https://example.test/stream", "jazz").unwrap();
        repo.add(station.clone()).unwrap();
        assert!(repo.add(station).is_err());
    }

    #[test]
    fn get_returns_matching_station() {
        let dir = TempDir::new().unwrap();
        let mut repo = open(&dir);
        let station = Station::new("Echo FM", "https://example.test/stream", "jazz").unwrap();
        let id = station.id.clone();
        repo.add(station.clone()).unwrap();
        assert_eq!(repo.get(&id), Some(station));
    }

    #[test]
    fn get_returns_none_for_unknown_id() {
        let dir = TempDir::new().unwrap();
        let repo = open(&dir);
        assert!(repo.get(&StationId::new("station-unknown")).is_none());
    }

    #[test]
    fn remove_deletes_station() {
        let dir = TempDir::new().unwrap();
        let mut repo = open(&dir);
        let station = Station::new("Echo FM", "https://example.test/stream", "jazz").unwrap();
        let id = station.id.clone();
        repo.add(station).unwrap();
        assert!(repo.remove(&id).unwrap());
        assert!(repo.get(&id).is_none());
    }

    #[test]
    fn remove_returns_false_for_unknown_id() {
        let dir = TempDir::new().unwrap();
        let mut repo = open(&dir);
        assert!(!repo.remove(&StationId::new("station-unknown")).unwrap());
    }

    #[test]
    fn persists_across_reopen() {
        let dir = TempDir::new().unwrap();
        let station = Station::new("Echo FM", "https://example.test/stream", "jazz").unwrap();
        let id = station.id.clone();

        {
            let mut repo = open(&dir);
            repo.add(station).unwrap();
        }

        let repo = open(&dir);
        assert_eq!(repo.get(&id).map(|s| s.name), Some("Echo FM".to_string()));
    }

    #[test]
    fn add_many_skips_duplicates_and_returns_count() {
        let dir = TempDir::new().unwrap();
        let mut repo = open(&dir);
        // KEXP is already seeded.
        let kexp = repo.list()[0].clone();
        let new_station =
            Station::new("Fresh Radio", "https://example.test/fresh", "indie").unwrap();
        let count = repo.add_many(vec![kexp, new_station]).unwrap();
        assert_eq!(count, 1);
    }
}
