use rand::Rng;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

pub type SpaceCoord = (u32, u32, u32);

mod planet;
mod station;

#[allow(dead_code)]
#[derive(Debug)]
pub enum SpaceObject {
    Star,
    BaseStation(station::Station),
    Planet(planet::Planet),
}

#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct Galaxy(Arc<RwLock<BTreeMap<u32, BTreeMap<u32, BTreeMap<u32, Arc<SpaceObject>>>>>>);

impl Galaxy {
    pub fn init() -> Galaxy {
        Galaxy(Arc::new(RwLock::new(BTreeMap::new())))
    }

    fn insert_space_object(&self, coord: SpaceCoord, obj: SpaceObject) {
        let mut galaxy = self.0.write().unwrap();
        if let Some(ref mut ydata) = galaxy.get_mut(&coord.0) {
            if let Some(ref mut zdata) = ydata.get_mut(&coord.1) {
                if zdata.get(&coord.2).is_some() {
                    panic!("Coordinate {coord:?} already taken, cannot insert object {obj:?}");
                } else {
                    zdata.insert(coord.2, Arc::new(obj));
                }
            } else {
                let mut zdata = BTreeMap::new();
                zdata.insert(coord.2, Arc::new(obj));
                ydata.insert(coord.1, zdata);
            }
        } else {
            let mut zdata = BTreeMap::new();
            zdata.insert(coord.2, Arc::new(obj));
            let mut ydata = BTreeMap::new();
            ydata.insert(coord.1, zdata);
            galaxy.insert(coord.0, ydata);
        }
    }

    // TODO (#11) Generate based on the galaxy
    pub fn init_new_station(&self) -> SpaceCoord {
        let mut rng = rand::rng();
        let coord = (rng.random(), rng.random(), rng.random());
        let station = station::Station::init();
        self.insert_space_object(coord, SpaceObject::BaseStation(station));
        coord
    }

    pub fn get_space_object(&self, coord: SpaceCoord) -> Option<Arc<SpaceObject>> {
        self.0
            .read()
            .unwrap()
            .get(&coord.0)?
            .get(&coord.1)?
            .get(&coord.2)
            .cloned()
    }
}
