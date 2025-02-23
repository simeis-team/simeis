use std::sync::{Arc, RwLock};

use serde_json::json;

use crate::api::ApiResult;
use crate::crew::Crew;
use crate::errors::Errcode;
use crate::player::Player;
use crate::ship::{Ship, ShipId};

pub struct Station {
    shipyard: Vec<Ship>,
    pub idle_crew: Crew,
}

impl Station {
    pub fn init() -> Station {
        Station {
            shipyard: Ship::init_shipyard(),
            idle_crew: Crew::default(),
        }
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
    let ship = station.write().unwrap().shipyard.remove(index);
    player.money -= price;
    player.ships.insert(id, ship);

    Ok(json!({}))
}
