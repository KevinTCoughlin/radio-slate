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
