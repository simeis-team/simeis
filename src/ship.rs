use cargo::ShipCargo;
use module::ShipModule;
use rand::Rng;
use serde::Serialize;
use shipstats::ShipStats;

use crate::api::ApiResult;
use crate::crew::{Crew, CrewId, CrewMemberType};
use crate::galaxy::SpaceCoord;

pub mod cargo;
pub mod module;
pub mod shipstats;

const FUEL_TANK_CAP_PRICE: f64 = 5.0;
const CARGO_CAP_PRICE: f64 = 10.0;
const HULL_DECAY_CAP_PRICE: f64 = 20.0;
const REACTOR_POWER_PRICE: f64 = 150.0;

pub type ShipId = u64;

#[derive(Serialize, Default)]
pub struct Ship {
    pub id: ShipId,
    position: SpaceCoord,

    pub modules: Vec<ShipModule>,
    pub crew: Crew,

    reactor_power: u16,

    cargo: ShipCargo,
    cargo_capacity: u64,

    fuel_tank: u64,
    fuel_tank_capacity: u64,

    hull_decay: u64,
    hull_decay_capacity: u64,

    pub pilot: Option<CrewId>,
    pub stats: shipstats::ShipStats,
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

    // Public data of this ship to display on the marketplace
    pub fn market_data(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "price": self.compute_price(),
            "modules": self.modules,
            "reactor_power": self.reactor_power,
            "cargo_capacity": self.cargo_capacity,
            "fuel_tank_capacity": self.fuel_tank_capacity,
            "hull_decay_capacity": self.hull_decay_capacity,
        })
    }

    pub fn compute_price(&self) -> f64 {
        let mut price = 0.0;
        price += (self.reactor_power as f64) * REACTOR_POWER_PRICE;
        price += (self.fuel_tank_capacity as f64) * FUEL_TANK_CAP_PRICE;
        price += (self.cargo_capacity as f64) * CARGO_CAP_PRICE;
        price += (self.hull_decay_capacity as f64) * HULL_DECAY_CAP_PRICE;
        price += self.modules.iter().map(|m| m.compute_price()).sum::<f64>();
        price
    }

    // Updates the performances of the ship based on the crew onboard
    pub fn update_perf_stats(&mut self) {
        self.stats = ShipStats::default();
        self.stats.speed = if let Some(ref pilot) = self.pilot {
            let pilot = self.crew.0.get(pilot).unwrap();
            debug_assert!(matches!(pilot.member_type, CrewMemberType::Pilot));
            (self.reactor_power as f64) * (pilot.rank as f64)
        } else {
            0.0
        };
        self.stats.speed *= 1.0 - self.cargo.slowing_ratio(self.cargo_capacity);

        let mut modules = self.modules.iter().collect::<Vec<&ShipModule>>();
        modules.sort_by_key(|a| a.priority());
        for smod in modules.into_iter().rev() {
            smod.apply_to_stats(&self.crew, &mut self.stats);
        }
    }
}

pub fn get_ship_status(ship: &Ship) -> ApiResult {
    Ok(serde_json::to_value(ship).unwrap())
}
