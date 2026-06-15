use crate::domain::{MutableStationRepository, Station, StationId, StationRepository};

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

impl MutableStationRepository for InMemoryStationRepository {
    fn add(&mut self, station: Station) -> anyhow::Result<()> {
        if self.stations.iter().any(|s| s.id == station.id) {
            anyhow::bail!(
                "station '{}' already exists in the library",
                station.id.as_ref()
            );
        }
        self.stations.push(station);
        Ok(())
    }

    fn remove(&mut self, id: &StationId) -> anyhow::Result<bool> {
        let before = self.stations.len();
        self.stations.retain(|s| s.id != *id);
        Ok(self.stations.len() < before)
    }

    fn add_many(&mut self, stations: Vec<Station>) -> anyhow::Result<usize> {
        let mut added = 0usize;
        for station in stations {
            if !self.stations.iter().any(|s| s.id == station.id) {
                self.stations.push(station);
                added += 1;
            }
        }
        Ok(added)
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
