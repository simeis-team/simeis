use serde::Serialize;

#[derive(Serialize)]
#[allow(dead_code)]
pub enum ShipModule {
    Miner,
}

impl ShipModule {
    pub fn compute_price(&self) -> f64 {
        0.0
    }
}
