use serde::Serialize;

use crate::crew::{Crew, CrewId, CrewMemberType};

use super::shipstats::ShipStats;

#[derive(Serialize)]
#[allow(dead_code)]
pub enum ShipModule {
    Miner(Option<CrewId>),
}

impl ShipModule {
    pub fn compute_price(&self) -> f64 {
        0.0
    }

    // Returns
    pub fn need(&self, ctype: &CrewMemberType) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match (self, ctype) {
            (ShipModule::Miner(None), CrewMemberType::Operator) => true,
            _ => false,
        }
    }

    pub fn set_crew(&mut self, id: CrewId, ctype: &CrewMemberType) {
        match (self, ctype) {
            (ShipModule::Miner(ref mut op), CrewMemberType::Operator) => *op = Some(id),
            _ => unreachable!(),
        }
    }

    // Define which module will be occupied by a crew member first
    pub fn priority(&self) -> u8 {
        match self {
            ShipModule::Miner(_) => u8::MAX,
        }
    }

    pub fn apply_to_stats(&self, crew: &Crew, stats: &mut ShipStats) {
        if let ShipModule::Miner(Some(id)) = self {
            let cm = crew.0.get(id).unwrap();
            debug_assert!(matches!(cm.member_type, CrewMemberType::Operator));
            stats.mining_force += (cm.rank as u32).pow(3);
        }
    }
}
