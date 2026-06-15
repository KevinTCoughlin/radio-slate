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
}
