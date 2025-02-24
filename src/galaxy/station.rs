use std::sync::{Arc, RwLock};

use serde::Serialize;
use serde_json::json;

use crate::api::ApiResult;
use crate::crew::Crew;
use crate::errors::Errcode;
use crate::player::Player;
use crate::ship::cargo::ShipCargo;
use crate::ship::{Ship, ShipId};

use super::scan::ScanResult;
use super::{Galaxy, SpaceCoord};

const CARGO_BASE_PRICE: f64 = 2.0;
// For X units of cargo purshased, price goes from (base ^ n) to (base ^ (n+1))
const CARGO_PRICE_INCDIV: f64 = 1000.0;

pub type StationId = u16;

#[derive(Serialize)]
pub struct Station {
    pub id: StationId,
    pub position: SpaceCoord,

    // Data that can't be scanned
    #[serde(skip)]
    pub idle_crew: Crew,
    #[serde(skip)]
    shipyard: Vec<Ship>,
    #[serde(skip)]
    pub cargo: ShipCargo,
}

impl Station {
    pub fn init(id: u16, position: super::SpaceCoord) -> Station {
        Station {
            id,
            position,
            idle_crew: Crew::default(),
            shipyard: Ship::init_shipyard(position),
            cargo: ShipCargo::with_capacity(0.0),
        }
    }

    // TODO (#27) Allow to build improvements for the scanner
    pub fn scan(&self, galaxy: &Galaxy) -> ScanResult {
        galaxy.scan_sector(&self.position, 0.0)
    }

    pub fn cargo_price(&self) -> f64 {
        CARGO_BASE_PRICE.powf(self.cargo.capacity / CARGO_PRICE_INCDIV)
    }

    pub fn buy_cargo(&mut self, player: &mut Player, amnt: &usize) -> Result<&ShipCargo, Errcode> {
        let cost = (*amnt as f64) * self.cargo_price();
        if cost > player.money {
            return Err(Errcode::NotEnoughMoney(player.money, cost));
        }
        player.money -= cost;
        self.cargo.capacity += *amnt as f64;

        Ok(&self.cargo)
    }
}

// TODO (#22)    Have a "ship price rate" metric for a station, that afffects the ship prices
//     Correlated to the price of the resources on the station

pub fn get_idle_crew(station: Arc<RwLock<Station>>) -> ApiResult {
    Ok(json!({"idle": station.read().unwrap().idle_crew}))
}

pub fn list_shipyard_ships(station: Arc<RwLock<Station>>) -> ApiResult {
    let ships = station
        .read()
        .unwrap()
        .shipyard
        .iter()
        .map(|ship| ship.market_data())
        .collect::<Vec<serde_json::Value>>();
    Ok(json!({
        "ships": ships,
    }))
}

// TODO (#22)    Allow to sell ships
pub fn buy_ship(
    player: Arc<RwLock<Player>>,
    station: Arc<RwLock<Station>>,
    id: ShipId,
) -> ApiResult {
    let ship_opt = {
        let mut data = None;
        for (n, ship) in station.read().unwrap().shipyard.iter().enumerate() {
            if ship.id == id {
                data = Some((n, ship.compute_price()));
            }
        }
        data
    };

    let Some((index, price)) = ship_opt else {
        return Err(Errcode::ShipNotFound(id));
    };

    let money_got = player.read().unwrap().money;
    if price > money_got {
        return Err(Errcode::NotEnoughMoney(money_got, price));
    }

    let mut player = player.write().unwrap();
    let mut ship = station.write().unwrap().shipyard.remove(index);
    ship.update_perf_stats();
    ship.fuel_tank = ship.fuel_tank_capacity;
    player.money -= price;
    player.ships.insert(id, ship);

    Ok(json!({}))
}
