use serde::Serialize;

use super::SpaceCoord;

#[derive(Serialize, Debug)]
pub struct Planet {
    position: SpaceCoord,
    temperature: u16,
    solid: bool,
}

impl Planet {
    pub fn random<R: rand::Rng>(coord: SpaceCoord, rng: &mut R) -> Planet {
        Planet {
            solid: rng.random_bool(0.4),
            temperature: rng.random(),
            position: coord,
        }
    }
}
