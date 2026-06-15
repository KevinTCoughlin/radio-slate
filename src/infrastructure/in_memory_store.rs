use crate::domain::{Station, StationId, StationRepository};

#[derive(Default, Clone)]
pub struct InMemoryStationRepository {
    stations: Vec<Station>,
}

impl InMemoryStationRepository {
    pub fn with_seed_stations(stations: Vec<Station>) -> Self {
        Self { stations }
    }
}

impl StationRepository for InMemoryStationRepository {
    fn list(&self) -> Vec<Station> {
        self.stations.clone()
    }

    fn get(&self, id: &StationId) -> Option<Station> {
        self.stations
            .iter()
            .find(|station| station.id == *id)
            .cloned()
    }

    fn add(&mut self, station: Station) -> Result<(), String> {
        if self.stations.iter().any(|s| s.id == station.id) {
            return Err(format!(
                "station '{}' already exists in the library",
                station.id.as_ref()
            ));
        }
        self.stations.push(station);
        Ok(())
    }

    fn remove(&mut self, id: &StationId) -> Result<bool, String> {
        let before = self.stations.len();
        self.stations.retain(|s| s.id != *id);
        Ok(self.stations.len() < before)
    }
}

#[cfg(test)]
mod tests {
    use super::InMemoryStationRepository;
    use crate::domain::{Station, StationId, StationRepository};

    fn make_station(name: &str, genre: &str) -> Station {
        Station::new(name, "https://example.test/stream", genre).unwrap()
    }

    #[test]
    fn empty_repository_returns_no_stations() {
        let repo = InMemoryStationRepository::default();
        assert!(repo.list().is_empty());
    }

    #[test]
    fn repository_lists_seeded_stations() {
        let stations = vec![make_station("Echo", "news"), make_station("Vibe", "jazz")];
        let repo = InMemoryStationRepository::with_seed_stations(stations);
        assert_eq!(repo.list().len(), 2);
    }

    #[test]
    fn repository_get_returns_matching_station() {
        let station = make_station("Echo", "news");
        let id = station.id.clone();
        let repo = InMemoryStationRepository::with_seed_stations(vec![station.clone()]);
        assert_eq!(repo.get(&id), Some(station));
    }

    #[test]
    fn repository_get_returns_none_for_unknown_id() {
        let repo =
            InMemoryStationRepository::with_seed_stations(vec![make_station("Echo", "news")]);
        assert!(repo.get(&StationId::new("station-unknown")).is_none());
    }

    #[test]
    fn add_inserts_station() {
        let mut repo = InMemoryStationRepository::default();
        let station = make_station("Echo", "news");
        repo.add(station.clone()).unwrap();
        assert_eq!(repo.list().len(), 1);
        assert_eq!(repo.get(&station.id), Some(station));
    }

    #[test]
    fn add_rejects_duplicate() {
        let mut repo = InMemoryStationRepository::default();
        let station = make_station("Echo", "news");
        repo.add(station.clone()).unwrap();
        assert!(repo.add(station).is_err());
    }

    #[test]
    fn remove_deletes_station() {
        let station = make_station("Echo", "news");
        let id = station.id.clone();
        let mut repo = InMemoryStationRepository::with_seed_stations(vec![station]);
        assert!(repo.remove(&id).unwrap());
        assert!(repo.get(&id).is_none());
    }

    #[test]
    fn remove_returns_false_for_unknown_id() {
        let mut repo = InMemoryStationRepository::default();
        assert!(!repo.remove(&StationId::new("station-unknown")).unwrap());
    }

    #[test]
    fn add_many_skips_duplicates_and_returns_count() {
        let station_a = make_station("Echo", "news");
        let station_b = make_station("Vibe", "jazz");
        let mut repo = InMemoryStationRepository::with_seed_stations(vec![station_a.clone()]);
        let count = repo.add_many(vec![station_a, station_b]).unwrap();
        assert_eq!(count, 1);
        assert_eq!(repo.list().len(), 2);
    }
}
