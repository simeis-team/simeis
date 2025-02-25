use std::collections::BTreeMap;

use serde::Serialize;

use super::resources::Resource;

#[derive(Default, Serialize)]
pub struct ShipCargo {
    pub capacity: f64,
    pub usage: f64,
    pub resources: BTreeMap<Resource, f64>,
}

impl ShipCargo {
    pub fn with_capacity(cap: f64) -> ShipCargo {
        ShipCargo {
            usage: 0.0,
            capacity: cap,
            resources: BTreeMap::new(),
        }
    }

    pub fn slowing_ratio(&self) -> f64 {
        // let usage_ratio = self.usage / self.capacity;
        // TODO (#12)    Cargo slows down speed of ship
        0.0
    }

    pub fn add_resource(&mut self, res: &Resource, mut amnt: f64) -> f64 {
        log::debug!("Added {amnt} {res:?} to cargo");
        let added = res.volume() * amnt;
        if self.usage == self.capacity {
            return 0.0;
        } else if (self.usage + added) > self.capacity {
            let overflow = ((self.usage + added) / self.capacity) - 1.0;
            amnt -= overflow * amnt;
            self.usage = self.capacity;
        } else {
            self.usage += added;
        }

        if let Some(stock) = self.resources.get_mut(res) {
            *stock += amnt;
        } else {
            self.resources.insert(*res, amnt);
        }
        amnt
    }

    pub fn is_full(&self) -> bool {
        self.usage == self.capacity
    }

    pub fn unload(&mut self, resource: &Resource, amnt: f64) -> f64 {
        if let Some(got) = self.resources.get_mut(resource) {
            let unload = got.min(amnt);
            log::debug!("{got:?} {amnt:?} {unload:?}");
            *got -= unload;
            self.usage -= resource.volume() * unload;
            debug_assert!(self.usage >= 0.0);
            unload
        } else {
            0.0
        }
    }

    // Compute how much of a resource we can store (based on its volume)
    pub fn space_for(&self, resource: &Resource) -> f64 {
        let capleft = self.capacity - self.usage;
        capleft / resource.volume()
    }
}
