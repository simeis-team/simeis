use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use strum::{EnumIter, EnumString, IntoStaticStr};

use crate::galaxy::planet::Planet;

use super::{cargo::ShipCargo, Ship};

#[derive(
    EnumIter,
    EnumString,
    IntoStaticStr,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Copy,
)]
#[strum(ascii_case_insensitive)]
pub enum Resource {
    // Solid or liquid
    Stone,
    Iron,

    // Gaseous
    Helium,
    Ozone,

    // Crafted
    Fuel,
    HullPlate,
}

impl Resource {
    // TODO (#24) Get from configuration
    #[inline]
    pub const fn base_price(&self) -> f64 {
        match self {
            Resource::Stone | Resource::Helium => 1.75,
            Resource::Iron | Resource::Ozone => 7.0,
            Resource::Fuel => 3.2,
            Resource::HullPlate => 1.8,
        }
    }

    pub fn volume(&self) -> f64 {
        match self {
            Resource::Stone | Resource::Helium => 0.75,
            Resource::Iron | Resource::Ozone => 0.3,

            Resource::Fuel => 2.0,
            Resource::HullPlate => 0.05,
        }
    }

    pub fn min_rank(&self) -> u8 {
        match self {
            Resource::Stone => 0,
            Resource::Iron => 3,
            Resource::Helium => 0,
            Resource::Ozone => 3,
            Resource::Fuel
            | Resource::HullPlate => 0,
        }
    }

    pub fn mineable(&self, rank: u8) -> bool {
        match self {
            Resource::Stone
            | Resource::Iron => rank > self.min_rank(),
            _ => false,
        }
    }

    pub fn suckable(&self, rank: u8) -> bool {
        match self {
            Resource::Helium => true,
            Resource::Ozone => rank > 3,
            _ => false,
        }
    }

    pub fn extraction_difficulty(&self) -> f64 {
        match self {
            Resource::Stone | Resource::Helium => 0.5,
            Resource::Iron | Resource::Ozone => 1.0,

            // All the things that are only crafted
            _ => unreachable!("Extraction difficulty on crafted resources"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ExtractionInfo(pub BTreeMap<Resource, f64>);
impl ExtractionInfo {
    pub fn create(ship: &Ship, planet: &Planet) -> Self {
        let mut extraction = BTreeMap::new();
        for (_, smod) in ship.modules.iter() {
            for (res, rate) in smod.can_extract(&ship.crew, planet) {
                if let Some(rrate) = extraction.get_mut(&res) {
                    *rrate += rate;
                } else {
                    extraction.insert(res, rate);
                }
            }
        }
        ExtractionInfo(extraction)
    }

    pub fn update_cargo(&self, cargo: &mut ShipCargo, tdelta: f64) -> bool {
        for (res, rate) in self.0.iter() {
            cargo.add_resource(res, *rate * tdelta);
        }
        cargo.is_full()
    }

    pub fn time_before_cargo_full(&self, cargocap: f64) -> std::time::Duration {
        let mut vol_per_sec = 0.0;
        for (res, rate) in self.0.iter() {
            vol_per_sec += res.volume() * rate;
        }
        std::time::Duration::from_secs_f64(cargocap / vol_per_sec)
    }
}
