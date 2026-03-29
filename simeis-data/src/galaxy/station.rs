use std::collections::BTreeMap;

use rand::RngExt;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::crew::{Crew, CrewId, CrewMember, CrewMemberType};
use crate::errors::Errcode;
use crate::market::{fee_rate, Market, MarketTx};
use crate::player::{Player, PlayerId};
use crate::ship::cargo::ShipCargo;
use crate::ship::module::ShipModuleId;
use crate::ship::resources::Resource;
use crate::ship::upgrade::ShipUpgrade;
use crate::ship::Ship;

use super::scan::ScanResult;
use super::{Galaxy, SpaceCoord};

const CARGO_BASE_PRICE: f64 = 2.0;
const CARGO_PRICE_INCDIV: f64 = 100.0;
pub const STATION_INIT_CARGO: f64 = 1000.0;

pub type StationId = u16;

// TODO (#7) Add refineries to create fuel & hull plate from raw resources
#[derive(Serialize, Deserialize, Debug)]
pub struct StationInfo {
    pub id: StationId,
    pub position: SpaceCoord,
}

impl StationInfo {
    // TODO (#8) Based on the scanner rank, get informations on crew and cargo
    pub fn scan(_rank: u8, station: &Station) -> StationInfo {
        StationInfo {
            id: station.id,
            position: station.position,
        }
    }
}

#[derive(Default, Debug, Serialize)]
pub struct StationPlayerData {
    pub idle_crew: Crew,
    pub crew: Crew,
    pub trader: Option<CrewId>,
    pub cargo: ShipCargo,
}

impl StationPlayerData {
    pub fn new() -> StationPlayerData {
        StationPlayerData {
            cargo: ShipCargo::with_capacity(STATION_INIT_CARGO),
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct Station {
    pub id: StationId,
    pub position: SpaceCoord,
    pub shipyard: Vec<Ship>,

    pub player_data: RwLock<BTreeMap<PlayerId, StationPlayerData>>,
}

impl Station {
    pub fn init(id: u16, position: super::SpaceCoord) -> Station {
        Station {
            id,
            position,
            shipyard: Ship::init_shipyard(position),
            player_data: RwLock::new(BTreeMap::new()),
        }
    }

    // TODO (#8) Allow to build improvements for the scanner
    pub async fn scan(&self, galaxy: &Galaxy) -> ScanResult {
        galaxy.scan_sector(1, &self.position).await
    }

    pub async fn cargo_price(&self, player: &PlayerId) -> f64 {
        let cap = if let Some(data) = self.player_data.read().await.get(player) {
            data.cargo.capacity
        } else {
            return STATION_INIT_CARGO;
        };
        CARGO_BASE_PRICE.powf((cap - STATION_INIT_CARGO) / CARGO_PRICE_INCDIV)
    }

    pub async fn buy_cargo(
        &mut self,
        player: &mut Player,
        amnt: &usize,
    ) -> Result<ShipCargo, Errcode> {
        let cost = (*amnt as f64) * self.cargo_price(&player.id).await;
        if cost > player.money {
            return Err(Errcode::NotEnoughMoney(player.money, cost));
        }
        player.money -= cost;
        self.ensure_has_player_data(&player.id).await;
        let mut apd = self.player_data.write().await;
        let pd = apd.get_mut(&player.id).unwrap();
        pd.cargo.capacity += *amnt as f64;
        Ok(pd.cargo.clone())
    }

    pub async fn assign_trader(&mut self, pid: &PlayerId, id: CrewId) -> Result<(), Errcode> {
        self.ensure_has_player_data(pid).await;
        let mut apd = self.player_data.write().await;
        let pd = apd.get_mut(pid).unwrap();
        let Some(cm) = pd.idle_crew.0.remove(&id) else {
            return Err(Errcode::CrewMemberNotIdle(id));
        };

        pd.crew.0.insert(id, cm);
        pd.trader = Some(id);
        Ok(())
    }

    pub async fn onboard_pilot(&mut self, id: CrewId, ship: &mut Ship) -> Result<(), Errcode> {
        self.ensure_has_player_data(&ship.owner).await;
        let mut apd = self.player_data.write().await;
        let pd = apd.get_mut(&ship.owner).unwrap();
        let Some(cm) = pd.idle_crew.0.get(&id) else {
            return Err(Errcode::CrewMemberNotIdle(id));
        };

        if cm.member_type != CrewMemberType::Pilot {
            return Err(Errcode::WrongCrewType(CrewMemberType::Pilot));
        }

        if ship.pilot.is_some() {
            return Err(Errcode::CrewNotNeeded);
        }
        ship.pilot = Some(id);
        ship.crew.0.insert(id, pd.idle_crew.0.remove(&id).unwrap());
        ship.update_perf_stats();
        Ok(())
    }

    pub async fn onboard_operator(
        &mut self,
        id: CrewId,
        ship: &mut Ship,
        modid: &ShipModuleId,
    ) -> Result<(), Errcode> {
        self.ensure_has_player_data(&ship.owner).await;
        let mut apd = self.player_data.write().await;
        let pd = apd.get_mut(&ship.owner).unwrap();
        let Some(cm) = pd.idle_crew.0.get(&id) else {
            return Err(Errcode::CrewMemberNotIdle(id));
        };

        if cm.member_type != CrewMemberType::Operator {
            return Err(Errcode::WrongCrewType(CrewMemberType::Pilot));
        }

        let Some(smod) = ship.modules.get_mut(modid) else {
            return Err(Errcode::NoSuchModule(*modid));
        };
        if !smod.need(&cm.member_type) {
            return Err(Errcode::CrewNotNeeded);
        }
        smod.operator = Some(id);
        ship.crew.0.insert(id, pd.idle_crew.0.remove(&id).unwrap());
        Ok(())
    }

    pub async fn buy_resource(
        &mut self,
        resource: &Resource,
        amnt: f64,
        player: &mut Player,
        market: &mut Market,
    ) -> Result<MarketTx, Errcode> {
        self.ensure_has_player_data(&player.id).await;
        let mut apd = self.player_data.write().await;
        let pd = apd.get_mut(&player.id).unwrap();
        let Some(trader) = pd.trader else {
            return Err(Errcode::NoTraderAssigned);
        };
        let cm = pd.crew.0.get(&trader).unwrap();
        let can_cargo = pd.cargo.space_for(resource);
        let amnt = amnt.min(can_cargo);
        if amnt == 0.0 {
            return Err(Errcode::BuyNothing);
        }

        let tx = market.buy(cm, resource, amnt);
        player.money -= tx.removed_money.unwrap();
        player.score -= tx.removed_money.unwrap();
        let (r, a) = tx.added_cargo.unwrap();
        pd.cargo.add_resource(&r, a);
        Ok(tx)
    }

    pub async fn sell_resource(
        &mut self,
        resource: &Resource,
        amnt: f64,
        player: &mut Player,
        market: &mut Market,
    ) -> Result<MarketTx, Errcode> {
        self.ensure_has_player_data(&player.id).await;
        let mut apd = self.player_data.write().await;
        let pd = apd.get_mut(&player.id).unwrap();
        let Some(trader) = pd.trader else {
            return Err(Errcode::NoTraderAssigned);
        };
        let cm = pd.crew.0.get(&trader).unwrap();
        let Some(can_cargo) = pd.cargo.resources.get(resource) else {
            return Err(Errcode::SellNothing);
        };
        let amnt = amnt.min(*can_cargo);
        if amnt <= 0.0 {
            return Err(Errcode::SellNothing);
        }

        let tx = market.sell(cm, resource, amnt);
        player.money += tx.added_money.unwrap();
        player.score += tx.added_money.unwrap();
        let (r, a) = tx.removed_cargo.unwrap();
        let unloaded = pd.cargo.unload(&r, a);
        debug_assert_eq!(unloaded, a);
        Ok(tx)
    }

    pub async fn refuel_ship(&mut self, ship: &mut Ship) -> Result<f64, Errcode> {
        if self.position != ship.position {
            return Err(Errcode::ShipNotInStation);
        }
        let mut apd = self.player_data.write().await;
        let Some(pd) = apd.get_mut(&ship.owner) else {
            return Err(Errcode::NoFuelInCargo);
        };
        let Some(qty) = pd.cargo.resources.get(&Resource::Fuel) else {
            return Err(Errcode::NoFuelInCargo);
        };
        if *qty == 0.0 {
            return Err(Errcode::NoFuelInCargo);
        }
        debug_assert!(ship.fuel_tank >= 0.0);
        debug_assert!(ship.fuel_tank_capacity >= ship.fuel_tank);
        let needed = ship.fuel_tank_capacity - ship.fuel_tank;
        let unloaded = pd.cargo.unload(&Resource::Fuel, needed.min(*qty));
        ship.fuel_tank += unloaded;
        debug_assert!(ship.fuel_tank_capacity >= ship.fuel_tank);
        Ok(unloaded)
    }

    pub async fn repair_ship(&mut self, ship: &mut Ship) -> Result<f64, Errcode> {
        if self.position != ship.position {
            return Err(Errcode::ShipNotInStation);
        }
        let mut apd = self.player_data.write().await;
        let Some(pd) = apd.get_mut(&ship.owner) else {
            return Err(Errcode::NoHullPlateInCargo);
        };
        let Some(qty) = pd.cargo.resources.get(&Resource::HullPlate) else {
            return Err(Errcode::NoHullPlateInCargo);
        };
        if *qty == 0.0 {
            return Err(Errcode::NoHullPlateInCargo);
        }
        debug_assert!(ship.hull_resistance >= ship.hull_decay);

        let amnt = ship.hull_decay.min(*qty);
        if amnt == 0.0 {
            return Ok(0.0);
        }
        let unloaded = pd.cargo.unload(&Resource::HullPlate, amnt);
        ship.hull_decay -= unloaded;
        debug_assert!(
            ship.hull_resistance >= ship.hull_decay,
            "{} < {}",
            ship.hull_resistance,
            ship.hull_decay
        );
        debug_assert!(ship.hull_decay >= 0.0, "{}", ship.hull_decay);
        debug_assert!(unloaded >= 0.0, "{}", unloaded);
        Ok(unloaded)
    }

    pub fn get_ship_upgrade_price(&self, _ship: &Ship, upgrade: &ShipUpgrade) -> f64 {
        // TODO (#9) Modify price based on station economy metrics
        // TODO (#9) Modify price based on upgrades already installed on the ship
        upgrade.get_price()
    }

    pub async fn get_cargo_potential_price(&self, id: &PlayerId) -> f64 {
        let apd = self.player_data.read().await;
        let Some(pd) = apd.get(id) else {
            return 0.0;
        };
        pd.cargo
            .resources
            .iter()
            .map(|(r, amnt)| r.base_price() * amnt)
            .sum()
    }

    pub async fn add_resource(&mut self, id: &PlayerId, resource: &Resource, amnt: f64) -> f64 {
        self.ensure_has_player_data(id).await;
        let mut apd = self.player_data.write().await;
        let pd = apd.get_mut(id).unwrap();
        pd.cargo.add_resource(resource, amnt)
    }

    pub fn buy_ship(&mut self, index: usize) -> Ship {
        // Ship starters, always keep them
        let mut ship = if index < 3 {
            self.shipyard.get(index).unwrap().clone()
        } else {
            let ship = self.shipyard.remove(index);
            self.shipyard.push(Ship::random(self.position));
            ship
        };
        ship.update_perf_stats();
        ship.fuel_tank = ship.fuel_tank_capacity;
        ship
    }

    pub async fn ensure_has_player_data(&self, id: &PlayerId) {
        let apd = self.player_data.read().await;
        if !apd.contains_key(id) {
            drop(apd);
            let mut apd = self.player_data.write().await;
            apd.insert(*id, StationPlayerData::new());
        }
    }

    pub async fn sum_all_wages(&self, id: &PlayerId) -> f64 {
        let apd = self.player_data.read().await;
        let Some(pd) = apd.get(id) else {
            return 0.0;
        };
        pd.crew.sum_wages() + pd.idle_crew.sum_wages()
    }

    pub async fn upgrade_station_crew(
        &mut self,
        id: &PlayerId,
        money: &mut f64,
        crew: &CrewId,
    ) -> Result<(f64, u8), Errcode> {
        let mut apd = self.player_data.write().await;
        let Some(pd) = apd.get_mut(id) else {
            return Err(Errcode::CrewMemberNotFound(*crew));
        };

        let Some(cm) = pd.crew.0.get_mut(crew) else {
            return Err(Errcode::CrewMemberNotFound(*crew));
        };
        let price = cm.price_next_rank();
        if price > *money {
            return Err(Errcode::NotEnoughMoney(*money, price));
        }
        *money -= price;
        cm.rank += 1;
        Ok((price, cm.rank))
    }

    pub async fn to_json(&self, id: &PlayerId) -> serde_json::Value {
        let apd = self.player_data.read().await;
        let pd = if let Some(pdr) = apd.get(id) {
            pdr
        } else {
            &StationPlayerData::new()
        };

        serde_json::json!({
            "id": self.id,
            "position": self.position,
            "crew": pd.crew,
            "cargo": pd.cargo,
            "idle_crew": pd.idle_crew,
            "trader": pd.trader,
        })
    }

    pub async fn hire_crew(&self, id: &PlayerId, crewtype: CrewMemberType) -> CrewId {
        let mut rng = rand::rng();
        let crewid = rng.random();
        let member = CrewMember::from(crewtype);

        self.ensure_has_player_data(id).await;
        let mut apd = self.player_data.write().await;
        let pd = apd.get_mut(id).unwrap();
        pd.idle_crew.0.insert(crewid, member);
        crewid
    }

    pub async fn upgr_trader_price(&self, id: &PlayerId) -> Option<f64> {
        let apd = self.player_data.read().await;
        let Some(pd) = apd.get(id) else {
            return None;
        };
        pd.trader.map(|trader| {
            let cm = pd.crew.0.get(&trader).unwrap();
            cm.price_next_rank()
        })
    }

    pub async fn clone_cargo(&self, id: &PlayerId) -> ShipCargo {
        let apd = self.player_data.read().await;
        if let Some(pd) = apd.get(id) {
            pd.cargo.clone()
        } else {
            ShipCargo::with_capacity(STATION_INIT_CARGO)
        }
    }

    pub async fn get_fee_rate(&self, id: &PlayerId) -> Result<f64, Errcode> {
        let apd = self.player_data.read().await;
        let Some(pd) = apd.get(id) else {
            return Err(Errcode::NoTraderAssigned);
        };
        let Some(trader) = pd.trader else {
            return Err(Errcode::NoTraderAssigned);
        };
        let cm = pd.crew.0.get(&trader).unwrap();
        Ok(fee_rate(cm.rank))
    }
}
