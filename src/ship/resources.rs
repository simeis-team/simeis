use serde::Serialize;
use strum::EnumIter;

#[derive(EnumIter, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Resource {
    // Solid or liquid
    Stone,
    Iron,

    // Gaseous
    Helium,
    Ozone,
}

impl Resource {
    pub fn from_str(s: &str) -> Option<Resource> {
        Some(match s {
            "stone" => Resource::Stone,
            "iron" => Resource::Iron,
            "helium" => Resource::Helium,
            "ozone" => Resource::Ozone,
            _ => return None,
        })
    }
    pub fn volume(&self) -> f64 {
        match self {
            Resource::Stone => 1.0,
            Resource::Iron => 0.5,
            Resource::Helium => 1.0,
            Resource::Ozone => 0.5,
        }
    }

    pub fn mineable(&self, rank: u8) -> bool {
        match self {
            Resource::Stone => true,
            Resource::Iron => rank > 1,
            _ => false,
        }
    }

    pub fn suckable(&self, rank: u8) -> bool {
        match self {
            Resource::Helium => true,
            Resource::Ozone => rank > 1,
            _ => false,
        }
    }

    pub fn extraction_difficulty(&self) -> f64 {
        match self {
            Resource::Stone => 1.0,
            Resource::Iron => 5.0,

            Resource::Helium => 1.0,
            Resource::Ozone => 5.0,
        }
    }
}
