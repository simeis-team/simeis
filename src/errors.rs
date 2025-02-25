#[derive(Debug)]
pub enum Errcode {
    NoPlayerKey,
    PlayerNotFound(crate::player::PlayerId),
    PlayerAlreadyExists(String),
    NoPlayerWithKey,
    ShipNotFound(crate::ship::ShipId),
    NotEnoughMoney(f64, f64),
    InvalidArgument(&'static str),
    ShipNotExtracting,
    ShipNotIdle,
    CrewMemberNotIdle(crate::crew::CrewId),
    CrewNotNeeded,
    CannotPerformTravel,
    NullDistance,
    NoSuchStation(crate::galaxy::station::StationId),
    NoSuchModule(crate::ship::module::ShipModuleId),
    CannotExtractWithoutPlanet,
    ShipNotInStation,
    WrongCrewType(crate::crew::CrewMemberType),
    CargoFull,
    NoTrader,
    BuyNothing,
    SellNothing,
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
            Errcode::CrewMemberNotIdle(id) => format!("Crew member {id} is already occupied"),
            Errcode::CrewNotNeeded => "This crew member is not needed aboard this ship".to_string(),
            Errcode::CannotPerformTravel => {
                "This travel cannot be done with the current state of the ship".to_string()
            }
            Errcode::NullDistance => "You already are on this coordinates".to_string(),
            Errcode::NoSuchStation(id) => format!("You don't own any station of id {id}"),
            Errcode::NoSuchModule(id) => format!("Ship module of id {id} doesn't exist"),
            Errcode::CannotExtractWithoutPlanet => {
                "Cannot extract resources, this ship is not on a planet".to_string()
            }
            Errcode::ShipNotInStation => "This ship is not docked on station".to_string(),
            Errcode::WrongCrewType(ctype) => {
                format!("This module requires a crew member of type {ctype:?}")
            }
            Errcode::CargoFull => "The cargo is full".to_string(),
            Errcode::ShipNotIdle => "The ship is already occupied with a task".to_string(),
            Errcode::ShipNotExtracting => "This ship is not extracting".to_string(),
            Errcode::NoTrader => "This station doesn't have a trader assigned".to_string(),
            Errcode::BuyNothing => "Either you attempted to BUY 0 units, or you don't have enough space in cargo to hold the resources".to_string(),
            Errcode::SellNothing => "Either you attempted to SELL 0 units, or you don't have any unit of this resource in your cargo".to_string(),
        }
    }
}
