use std::ops::Deref;
use std::sync::{Arc, RwLock};

use rand::Rng;
use serde::Serialize;

use crate::api::ApiResult;
use crate::galaxy::station::Station;
use crate::player::Player;

pub type CrewId = u32;

#[allow(dead_code)]
#[derive(Serialize)]
pub enum CrewType {
    Pilot,
    Operator,
    Trader,
    Soldier,
}

impl CrewType {
    pub fn wage(&self) -> f64 {
        match self {
            CrewType::Pilot => 10.0,
            CrewType::Operator => 1.0,
            CrewType::Trader => 5.0,
            CrewType::Soldier => 3.0,
        }
    }
}

pub fn hire_crew(
    player: Arc<RwLock<Player>>,
    station: Arc<RwLock<Station>>,
    crewtype: CrewType,
) -> ApiResult {
    let mut rng = rand::rng();
    let id = rng.random();
    station.write().unwrap().idle_crew.insert(id, crewtype);
    player
        .write()
        .unwrap()
        .update_wages(station.read().unwrap().deref());

    Ok(serde_json::json!({
        "crew_member_id": id,
        "idle": station.read().unwrap().idle_crew,
    }))
}
