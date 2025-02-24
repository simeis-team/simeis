use std::{
    ops::Deref,
    sync::{Arc, RwLock},
};

use super::{planet::Planet, station::Station, SpaceObject};

pub struct ScanResult {
    planets: Vec<Arc<RwLock<Planet>>>,
    stations: Vec<Arc<RwLock<Station>>>,
}

impl ScanResult {
    pub fn empty() -> ScanResult {
        ScanResult {
            planets: vec![],
            stations: vec![],
        }
    }

    pub fn add(&mut self, obj: &SpaceObject) {
        match obj {
            SpaceObject::BaseStation(station) => self.stations.push(station.clone()),
            SpaceObject::Planet(planet) => self.planets.push(planet.clone()),
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "planets": self.planets
                .iter()
                .map(|p| serde_json::to_value(p.read().unwrap().deref()).unwrap())
                .collect::<Vec<serde_json::Value>>(),
            "stations": self.stations
                .iter()
                .map(|s| serde_json::to_value(s.read().unwrap().deref()).unwrap())
                .collect::<Vec<serde_json::Value>>(),
        })
    }
}
