use std::collections::BTreeMap;

use cargo::ShipCargo;
use module::ShipModule;
use rand::Rng;
use serde::Serialize;

use crate::crew::{CrewId, CrewType};
use crate::galaxy::SpaceCoord;

pub mod cargo;
pub mod module;

const FUEL_TANK_CAP_PRICE: f64 = 5.0;
const CARGO_CAP_PRICE: f64 = 10.0;
const HULL_DECAY_CAP_PRICE: f64 = 20.0;
const REACTOR_POWER_PRICE: f64 = 150.0;

pub type ShipId = u64;

#[derive(Serialize, Default)]
pub struct Ship {
    pub id: ShipId,
    position: SpaceCoord,

    modules: Vec<ShipModule>,
    pub crew: BTreeMap<CrewId, CrewType>,

    reactor_power: u16,

    cargo: ShipCargo,
    cargo_capacity: u64,

    fuel_tank: u64,
    fuel_tank_capacity: u64,

    hull_decay: u64,
    hull_decay_capacity: u64,
}

impl Ship {
    pub fn init_shipyard() -> Vec<Ship> {
        let mut rng = rand::rng();
        vec![
            Ship::light(rng.random()),
            Ship::medium(rng.random()),
            Ship::heavy(rng.random()),
        ]
    }

    fn light(id: ShipId) -> Ship {
        Ship {
            id,
            reactor_power: 1,
            fuel_tank_capacity: 100,
            cargo_capacity: 500,
            hull_decay_capacity: 1000,
            ..Default::default()
        }
    }

    fn medium(id: ShipId) -> Ship {
        Ship {
            id,
            reactor_power: 3,
            fuel_tank_capacity: 200,
            cargo_capacity: 1000,
            hull_decay_capacity: 2000,
            ..Default::default()
        }
    }

    fn heavy(id: ShipId) -> Ship {
        Ship {
            id,
            reactor_power: 10,
            fuel_tank_capacity: 400,
            cargo_capacity: 3000,
            hull_decay_capacity: 5000,
            ..Default::default()
        }
    }

    // TODO (#22) Create a new ship with random specs
    //         Used by traders to seek nice ships to buy

    pub fn compute_price(&self) -> f64 {
        let mut price = 0.0;
        price += (self.reactor_power as f64) * REACTOR_POWER_PRICE;
        price += (self.fuel_tank_capacity as f64) * FUEL_TANK_CAP_PRICE;
        price += (self.cargo_capacity as f64) * CARGO_CAP_PRICE;
        price += (self.hull_decay_capacity as f64) * HULL_DECAY_CAP_PRICE;
        price += self.modules.iter().map(|m| m.compute_price()).sum::<f64>();
        price
    }

    pub fn to_json(&self) -> serde_json::Value {
        let mut data = serde_json::to_value(self).unwrap();
        crate::api::jsonmerge(
            &mut data,
            &serde_json::json!({
                "price": self.compute_price(),
            }),
        );
        data
    }
}
