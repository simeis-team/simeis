use serde::Serialize;

#[derive(Serialize, Default)]
pub struct ShipStats {
    pub speed: f64,

    pub mining_force: u32,
    pub mining_volume: u32,
    pub mining_speed: u32,
    pub mining_ore_scan: u32,
}
