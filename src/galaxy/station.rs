use std::sync::{Arc, RwLock};

use serde::Serialize;
use serde_json::json;

use crate::api::ApiResult;
use crate::crew::{Crew, CrewId};
use crate::errors::Errcode;
use crate::market::{Market, MarketTx};
use crate::player::Player;
use crate::ship::cargo::ShipCargo;
use crate::ship::resources::Resource;
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
    pub crew: Crew,
    #[serde(skip)]
    shipyard: Vec<Ship>,
    #[serde(skip)]
    pub cargo: ShipCargo,
    #[serde(skip)]
    pub trader: Option<CrewId>,
}

impl Station {
    pub fn init(id: u16, position: super::SpaceCoord) -> Station {
        Station {
            id,
            position,
            idle_crew: Crew::default(),
            crew: Crew::default(),
            shipyard: Ship::init_shipyard(position),
            cargo: ShipCargo::with_capacity(0.0),
            trader: None,
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

    pub fn assign_trader(&mut self, id: CrewId) -> Result<(), Errcode> {
        let Some(cm) = self.idle_crew.0.remove(&id) else {
            return Err(Errcode::CrewMemberNotIdle(id));
        };

        self.crew.0.insert(id, cm);
        self.trader = Some(id);
        Ok(())
    }

    pub fn buy_resource(
        &mut self,
        resource: &Resource,
        amnt: f64,
        player: &mut Player,
        market: &mut Market,
    ) -> Result<MarketTx, Errcode> {
        let Some(trader) = self.trader else {
            return Err(Errcode::NoTrader);
        };
        let cm = self.crew.0.get(&trader).unwrap();
        let can_cargo = self.cargo.space_for(resource);
        let amnt = amnt.min(can_cargo);
        if amnt == 0.0 {
            return Err(Errcode::BuyNothing);
        }

        let tx = market.buy(cm, resource, amnt);
        player.money -= tx.removed_money.unwrap();
        let (r, a) = tx.added_cargo.unwrap();
        self.cargo.add_resource(&r, a);
        Ok(tx)
    }

    pub fn sell_resource(
        &mut self,
        resource: &Resource,
        amnt: f64,
        player: &mut Player,
        market: &mut Market,
    ) -> Result<MarketTx, Errcode> {
        let Some(trader) = self.trader else {
            return Err(Errcode::NoTrader);
        };
        let cm = self.crew.0.get(&trader).unwrap();
        let Some(can_cargo) = self.cargo.resources.get(resource) else {
            return Err(Errcode::SellNothing);
        };
        let amnt = amnt.min(*can_cargo);
        if amnt == 0.0 {
            return Err(Errcode::SellNothing);
        }

        let tx = market.sell(cm, resource, amnt);
        player.money += tx.added_money.unwrap();
        let (r, a) = tx.removed_cargo.unwrap();
        let unloaded = self.cargo.unload(&r, a);
        debug_assert_eq!(unloaded, a);
        Ok(tx)
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
