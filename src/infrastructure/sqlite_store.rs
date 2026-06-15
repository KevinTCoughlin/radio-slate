use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

use crate::domain::{MutableStationRepository, Station, StationId, StationRepository};

const DEFAULT_STATION_NAME: &str = "KEXP";
const DEFAULT_STATION_URL: &str = "http://live-mp3-128.kexp.org/kexp128.mp3";
const DEFAULT_STATION_GENRE: &str = "eclectic";
const SCHEMA_VERSION: u32 = 1;

/// A durable station repository backed by an SQLite database.
///
/// On first use (empty database) the KEXP stream is seeded as a built-in
/// fallback so the app is usable out of the box.  A `schema_version` table
/// records the current schema version, providing a hook for future migrations.
///
/// The database is opened with the `bundled` SQLite feature, so no system
/// SQLite library is required at runtime.
pub struct SqliteStationRepository {
    conn: Connection,
}

impl SqliteStationRepository {
    /// Open (or create) the SQLite database at `path`, initialise the schema,
    /// and seed KEXP on a brand-new database.
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        let repo = Self { conn };
        repo.init_schema()?;
        Ok(repo)
    }

    /// Return the platform-appropriate default path for the SQLite database:
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

    fn init_schema(&self) -> anyhow::Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS stations (
                id    TEXT PRIMARY KEY,
                name  TEXT NOT NULL,
                url   TEXT NOT NULL,
                genre TEXT NOT NULL
            );
            ",
        )?;

        // Insert the schema version row only if the table is still empty.
        let version_count: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM schema_version",
            [],
            |row| row.get(0),
        )?;
        if version_count == 0 {
            self.conn.execute(
                "INSERT INTO schema_version (version) VALUES (?1)",
                params![SCHEMA_VERSION],
            )?;
        }

        // Seed KEXP on a brand-new station table.
        let station_count: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM stations",
            [],
            |row| row.get(0),
        )?;
        if station_count == 0 {
            let kexp = Station::new(
                DEFAULT_STATION_NAME,
                DEFAULT_STATION_URL,
                DEFAULT_STATION_GENRE,
            )
            .map_err(|e| anyhow::anyhow!(e))?;
            self.conn.execute(
                "INSERT INTO stations (id, name, url, genre) VALUES (?1, ?2, ?3, ?4)",
                params![kexp.id.as_ref(), kexp.name, kexp.url, kexp.genre],
            )?;
        }

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
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, url, genre FROM stations ORDER BY rowid")
            .expect("failed to prepare SELECT statement");
        stmt.query_map([], Self::row_to_station)
            .expect("failed to execute SELECT")
            .filter_map(|r| r.ok())
            .collect()
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
}

impl MutableStationRepository for SqliteStationRepository {
    fn add(&mut self, station: Station) -> anyhow::Result<()> {
        let rows = self.conn.execute(
            "INSERT OR IGNORE INTO stations (id, name, url, genre) VALUES (?1, ?2, ?3, ?4)",
            params![station.id.as_ref(), station.name, station.url, station.genre],
        )?;
        if rows == 0 {
            anyhow::bail!(
                "station '{}' already exists in the library",
                station.id.as_ref()
            );
        }
        Ok(())
    }

    fn remove(&mut self, id: &StationId) -> anyhow::Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM stations WHERE id = ?1", params![id.as_ref()])?;
        Ok(rows > 0)
    }

    fn add_many(&mut self, stations: Vec<Station>) -> anyhow::Result<usize> {
        let tx = self.conn.transaction()?;
        let mut added = 0usize;
        for station in stations {
            let rows = tx.execute(
                "INSERT OR IGNORE INTO stations (id, name, url, genre) VALUES (?1, ?2, ?3, ?4)",
                params![station.id.as_ref(), station.name, station.url, station.genre],
            )?;
            added += rows;
        }
        tx.commit()?;
        Ok(added)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp_db(dir: &TempDir) -> PathBuf {
        dir.path().join("stations.db")
    }

    #[test]
    fn new_database_seeds_kexp_fallback() {
        let dir = TempDir::new().unwrap();
        let repo = SqliteStationRepository::open(tmp_db(&dir)).unwrap();
        let stations = repo.list();
        assert_eq!(stations.len(), 1);
        assert_eq!(stations[0].name, DEFAULT_STATION_NAME);
        assert_eq!(stations[0].url, DEFAULT_STATION_URL);
    }

    #[test]
    fn add_and_get_round_trips_station() {
        let dir = TempDir::new().unwrap();
        let path = tmp_db(&dir);
        let station =
            Station::new("Test FM", "https://example.test/stream", "pop").unwrap();
        let mut repo = SqliteStationRepository::open(&path).unwrap();
        repo.add(station.clone()).unwrap();

        let reloaded = SqliteStationRepository::open(&path).unwrap();
        assert!(reloaded.list().iter().any(|s| s.id == station.id));
    }

    #[test]
    fn add_rejects_duplicate_station() {
        let dir = TempDir::new().unwrap();
        let mut repo = SqliteStationRepository::open(tmp_db(&dir)).unwrap();
        let station =
            Station::new("Echo", "https://example.test/stream", "jazz").unwrap();
        repo.add(station.clone()).unwrap();
        assert!(repo.add(station).is_err());
    }

    #[test]
    fn remove_deletes_station_and_persists() {
        let dir = TempDir::new().unwrap();
        let path = tmp_db(&dir);
        let station =
            Station::new("Echo", "https://example.test/stream", "jazz").unwrap();
        let id = station.id.clone();

        let mut repo = SqliteStationRepository::open(&path).unwrap();
        repo.add(station).unwrap();
        assert!(repo.remove(&id).unwrap());
        assert!(repo.get(&id).is_none());

        let reloaded = SqliteStationRepository::open(&path).unwrap();
        assert!(reloaded.get(&id).is_none());
    }

    #[test]
    fn remove_returns_false_for_unknown_id() {
        let dir = TempDir::new().unwrap();
        let mut repo = SqliteStationRepository::open(tmp_db(&dir)).unwrap();
        let id = StationId::new("station-nonexistent");
        assert!(!repo.remove(&id).unwrap());
    }

    #[test]
    fn add_many_skips_duplicates_and_returns_count() {
        let dir = TempDir::new().unwrap();
        let mut repo = SqliteStationRepository::open(tmp_db(&dir)).unwrap();
        let kexp = repo.list()[0].clone();
        let new_station =
            Station::new("Fresh Radio", "https://example.test/fresh", "indie").unwrap();
        let count = repo.add_many(vec![kexp, new_station]).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn schema_version_table_is_present_and_set() {
        let dir = TempDir::new().unwrap();
        let repo = SqliteStationRepository::open(tmp_db(&dir)).unwrap();
        let version: u32 = repo
            .conn
            .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn list_returns_stations_in_insertion_order() {
        let dir = TempDir::new().unwrap();
        let mut repo = SqliteStationRepository::open(tmp_db(&dir)).unwrap();
        let a = Station::new("Alpha", "https://alpha.test/stream", "rock").unwrap();
        let b = Station::new("Beta", "https://beta.test/stream", "jazz").unwrap();
        repo.add(a.clone()).unwrap();
        repo.add(b.clone()).unwrap();
        let list = repo.list();
        // KEXP is first (seeded), then Alpha, then Beta
        assert_eq!(list[1].id, a.id);
        assert_eq!(list[2].id, b.id);
    }
}
