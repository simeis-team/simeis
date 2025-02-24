use base64::prelude::{Engine, BASE64_STANDARD};
use rand::RngCore;
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hasher};
use std::ops::Deref;
use std::sync::{Arc, RwLock};

use crate::api::ApiResult;
use crate::errors::Errcode;
use crate::galaxy::station::{Station, StationId};
use crate::galaxy::SpaceCoord;
use crate::ship::{Ship, ShipId};
use crate::GameState;

const INIT_MONEY: f64 = 30000.0;

pub type PlayerId = u16;
pub type PlayerKey = [u8; 128];

// Game state for a single player
#[allow(dead_code)] // DEV
pub struct Player {
    pub id: PlayerId,
    key: PlayerKey,
    lost: bool,

    pub name: String,
    pub money: f64,
    pub costs: f64,

    pub stations: BTreeMap<StationId, SpaceCoord>,
    pub ships: BTreeMap<ShipId, Ship>,
}

impl Player {
    pub fn new(station: (StationId, SpaceCoord), req: ReqNewPlayer) -> Player {
        let mut hasher = DefaultHasher::new();
        hasher.write(req.name.as_bytes());
        let mut rng = rand::rng();
        let mut randbytes = [0; 128];
        rng.fill_bytes(&mut randbytes);

        #[allow(unused_mut)]
        let mut money = INIT_MONEY;

        #[cfg(feature = "testing")]
        if req.name.starts_with("test-rich") {
            money *= 10000.0;
        }
        let mut stations = BTreeMap::new();
        stations.insert(station.0, station.1);
        Player {
            key: randbytes,
            id: (hasher.finish() % (PlayerId::MAX as u64)) as PlayerId,
            lost: false,

            money,
            costs: 0.0,

            name: req.name,
            stations,
            ships: BTreeMap::new(),
        }
    }

    pub fn update_wages(&mut self, station: &Station) {
        self.costs = 0.0;
        self.costs += station.idle_crew.sum_wages();
        self.costs += self
            .ships
            .values()
            .map(|ship| ship.crew.sum_wages())
            .sum::<f64>();
    }

    pub fn update_money(&mut self, tdelta: f64) {
        self.money -= self.costs * tdelta;
        if self.money < 0.0 {
            self.lost = true;
            // TODO (#19)  What to do with its resources, ships, etc...
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ReqNewPlayer {
    name: String,
}

impl PartialEq<&Player> for ReqNewPlayer {
    fn eq(&self, other: &&Player) -> bool {
        self.name == other.name
    }
}

pub fn new_player(srv: GameState, req: ReqNewPlayer) -> ApiResult {
    for (_, player) in srv.players.read().unwrap().iter() {
        if req == player.read().unwrap().deref() {
            return Err(Errcode::PlayerAlreadyExists(req.name));
        }
    }

    let station = srv.galaxy.init_new_station();
    let player = Player::new(station, req);
    let resp = json!({
        "playerId": player.id,
        "key": &BASE64_STANDARD.encode(player.key),
    });
    srv.player_index
        .write()
        .unwrap()
        .insert(player.key, player.id);
    srv.players
        .write()
        .unwrap()
        .insert(player.id, Arc::new(RwLock::new(player)));
    Ok(resp)
}

pub fn get_player(srv: GameState, id: PlayerId, key: PlayerKey) -> ApiResult {
    let players = srv.players.read().unwrap();
    let Some(playerlck) = players.get(&id) else {
        return Err(Errcode::PlayerNotFound(id));
    };

    let player = playerlck.read().unwrap();

    #[allow(clippy::if_same_then_else)] // DEV
    if player.key == key {
        Ok(json!({
            "name": player.name,
            "stations": player.stations,
            "money": player.money,
            "ships": serde_json::to_value(
                player.ships.values().collect::<Vec<&Ship>>()
            ).unwrap(),
            "costs": player.costs,
        }))
    } else {
        Ok(json!({
            "name": player.name,
            "stations": player.stations,
        }))
    }
}
