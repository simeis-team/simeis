use std::sync::{Arc, RwLock};

use rand::Rng;
use simeis_data::{
    crew::{CrewMember, CrewMemberType},
    galaxy::{station::Station, Galaxy},
    player::Player,
};

use crate::api::ApiResult;

pub fn hire_crew(
    galaxy: &Galaxy,
    player: Arc<RwLock<Player>>,
    station: Arc<RwLock<Station>>,
    crewtype: CrewMemberType,
) -> ApiResult {
    let mut rng = rand::rng();
    let id = rng.random();
    let member = CrewMember::from(crewtype);
    station.write().unwrap().idle_crew.0.insert(id, member);
    player.write().unwrap().update_wages(galaxy);
    Ok(serde_json::json!({ "id": id }))
}
