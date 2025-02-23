use serde_json::json;

use crate::build_response;

#[derive(Debug)]
pub enum Errcode {
    NoPlayerKey,
    PlayerNotFound(u64),
    PlayerAlreadyExists(String),
}

impl Errcode {
    pub fn build_resp(&self) -> ntex::web::HttpResponse {
        let msg = match self {
            Errcode::NoPlayerKey => "No player key provided with the request".to_string(),
            Errcode::PlayerNotFound(id) => format!("No player was found with this ID: {id}"),
            Errcode::PlayerAlreadyExists(name) => format!("Player {name} already exists"),
        };

        build_response(json!({
            "type": format!("{self:?}"),
            "error": msg,
        }))
    }
}
