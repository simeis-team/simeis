use serde::Serialize;

#[derive(Default, Serialize)]
pub struct ShipCargo {}
impl ShipCargo {
    pub fn slowing_ratio(&self, _capacity: u64) -> f64 {
        // TODO (#12)    Cargo slows down speed of ship
        0.0
    }
}
