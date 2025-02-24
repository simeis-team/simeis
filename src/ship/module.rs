use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

use super::resources::Resource;
use crate::crew::{Crew, CrewId, CrewMemberType};
use crate::galaxy::planet::Planet;

pub type ShipModuleId = u16;

#[derive(EnumIter, Debug, Serialize, Deserialize)]
pub enum ShipModuleType {
    Miner,
    GasSucker,
    CargoExtension,
}

impl ShipModuleType {
    pub fn from_str(s: &str) -> Option<ShipModuleType> {
        Some(match s {
            "miner" => ShipModuleType::Miner,
            "gassucker" => ShipModuleType::GasSucker,
            "cargoext" => ShipModuleType::CargoExtension,
            _ => return None,
        })
    }
    pub fn new_module(self) -> ShipModule {
        ShipModule {
            operator: None,
            modtype: self,
        }
    }

    pub fn get_price_buy(&self) -> f64 {
        match self {
            ShipModuleType::Miner => 1000.0,
            ShipModuleType::GasSucker => 2000.0,
            ShipModuleType::CargoExtension => 5000.0,
        }
    }
}

#[derive(Serialize)]
pub struct ShipModule {
    pub operator: Option<CrewId>,
    pub modtype: ShipModuleType,
}

impl ShipModule {
    pub fn compute_price(&self) -> f64 {
        0.0
    }

    // Returns
    pub fn need(&self, ctype: &CrewMemberType) -> bool {
        match self.modtype {
            ShipModuleType::Miner | ShipModuleType::GasSucker => {
                ctype == &CrewMemberType::Operator && self.operator.is_none()
            }
            ShipModuleType::CargoExtension => false,
        }
    }

    pub fn can_extract(&self, crew: &Crew, planet: &Planet) -> Vec<(Resource, f64)> {
        let Some(ref opid) = self.operator else {
            log::debug!("No operator");
            return vec![];
        };

        let cm = crew.0.get(opid).unwrap();
        let all_resources = Resource::iter().filter(|r| planet.resource_present(r));
        match self.modtype {
            ShipModuleType::Miner => all_resources
                .filter(|r| r.mineable(cm.rank))
                .map(|r| (r, self.extraction_rate(&r, cm.rank)))
                .collect(),
            ShipModuleType::GasSucker => all_resources
                .filter(|r| r.suckable(cm.rank))
                .map(|r| (r, self.extraction_rate(&r, cm.rank)))
                .collect(),
            _ => vec![],
        }
    }

    pub fn extraction_rate(&self, resource: &Resource, oprank: u8) -> f64 {
        let d = resource.extraction_difficulty();
        1.0 / (d / (oprank as f64))
    }
}
