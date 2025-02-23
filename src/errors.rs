#[derive(Debug)]
pub enum Errcode {
    NoPlayerKey,
    PlayerNotFound(u64),
    PlayerAlreadyExists(String),
    NoPlayerWithKey,
    ShipNotFound(crate::ship::ShipId),
    NotEnoughMoney(f64, f64),
    InvalidArgument(&'static str),
    CrewMemberNotIdle(crate::crew::CrewId),
    ShipAlreadyHasPilot,
    CrewNotNeeded,
}

impl Errcode {
    pub fn errmsg(&self) -> String {
        match self {
            Errcode::NoPlayerKey => "No player key provided with the request".to_string(),
            Errcode::PlayerNotFound(id) => format!("No player was found with this ID: {id}"),
            Errcode::PlayerAlreadyExists(name) => format!("Player {name} already exists"),
            Errcode::NoPlayerWithKey => "No player with this key exists in this game".to_string(),
            Errcode::ShipNotFound(id) => format!("Ship of id {id} not found"),
            Errcode::NotEnoughMoney(got, need) => {
                format!("Not enough money, need {need}, got {got}")
            }
            Errcode::InvalidArgument(arg) => format!("Argument {arg} has an invalid value"),
            Errcode::CrewMemberNotIdle(_) => todo!(),
            Errcode::ShipAlreadyHasPilot => "This ship already has a pilot".to_string(),
            Errcode::CrewNotNeeded => "This crew member is not needed aboard this ship".to_string(),
        }
    }
}
