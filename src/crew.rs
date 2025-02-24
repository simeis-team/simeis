use std::collections::BTreeMap;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

use rand::Rng;
use serde::Serialize;

use crate::api::ApiResult;
use crate::errors::Errcode;
use crate::galaxy::station::Station;
use crate::player::Player;
use crate::ship::module::ShipModuleId;
use crate::ship::Ship;

pub type CrewId = u32;

#[derive(Default, Serialize)]
pub struct Crew(pub BTreeMap<CrewId, CrewMember>);
impl Crew {
    pub fn onboard(
        &mut self,
        id: CrewId,
        ship: &mut Ship,
        modid: &ShipModuleId,
    ) -> Result<(), Errcode> {
        let Some(cm) = self.0.get(&id) else {
            return Err(Errcode::CrewMemberNotIdle(id));
        };
        if *modid == 0 {
            if cm.member_type == CrewMemberType::Pilot {
                if ship.pilot.is_some() {
                    return Err(Errcode::CrewNotNeeded);
                }
                ship.pilot = Some(id);
                ship.crew.0.insert(id, self.0.remove(&id).unwrap());
                ship.update_perf_stats();
                return Ok(());
            } else {
                return Err(Errcode::WrongCrewType(CrewMemberType::Pilot));
            }
        }

        let Some(smod) = ship.modules.get_mut(modid) else {
            return Err(Errcode::NoSuchModule(*modid));
        };
        if !smod.need(&cm.member_type) {
            return Err(Errcode::CrewNotNeeded);
        }
        smod.operator = Some(id);
        ship.crew.0.insert(id, self.0.remove(&id).unwrap());
        Ok(())
    }

    pub fn sum_wages(&self) -> f64 {
        self.0.values().map(|crew| crew.wage()).sum::<f64>()
    }
}

#[derive(Serialize)]
pub struct CrewMember {
    pub member_type: CrewMemberType,
    pub rank: u8,
}

impl From<CrewMemberType> for CrewMember {
    fn from(member_type: CrewMemberType) -> Self {
        CrewMember {
            member_type,
            rank: 1,
        }
    }
}

impl CrewMember {
    pub fn wage(&self) -> f64 {
        let base = match self.member_type {
            CrewMemberType::Pilot => 10.0,
            CrewMemberType::Operator => 1.0,
            CrewMemberType::Trader => 5.0,
            CrewMemberType::Soldier => 3.0,
        };
        // TODO (#17)    Make the wage increase faster than rank
        base * (self.rank as f64)
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub enum CrewMemberType {
    Pilot,
    Operator,
    Trader,
    Soldier,
}

impl CrewMemberType {
    pub fn from_str(s: &str) -> Option<CrewMemberType> {
        Some(match s {
            "pilot" => CrewMemberType::Pilot,
            "trader" => CrewMemberType::Trader,
            "soldier" => CrewMemberType::Soldier,
            "operator" => CrewMemberType::Operator,
            _ => return None,
        })
    }
}

pub fn assign_crew_member(
    id: CrewId,
    station: Arc<RwLock<Station>>,
    ship: &mut Ship,
    modid: &ShipModuleId,
) -> ApiResult {
    station
        .write()
        .unwrap()
        .idle_crew
        .onboard(id, ship, modid)?;
    Ok(serde_json::json!({}))
}

pub fn hire_crew(
    player: Arc<RwLock<Player>>,
    station: Arc<RwLock<Station>>,
    crewtype: CrewMemberType,
) -> ApiResult {
    let mut rng = rand::rng();
    let id = rng.random();
    let member = CrewMember::from(crewtype);
    station.write().unwrap().idle_crew.0.insert(id, member);
    player
        .write()
        .unwrap()
        .update_wages(station.read().unwrap().deref());

    Ok(serde_json::json!({ "id": id }))
}
