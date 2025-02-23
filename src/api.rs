use base64::{prelude::BASE64_STANDARD, Engine};
use ntex::web::types::{Json, Path};
use ntex::web::{self, HttpRequest, HttpResponse, ServiceConfig};
use serde_json::{json, Value};

use crate::errors::Errcode;
use crate::player::{PlayerKey, ReqNewPlayer};
use crate::GameState;

pub type ApiResult = Result<serde_json::Value, Errcode>;

macro_rules! get_player {
    ($srv:ident, $req:ident) => {{
        let Some(key) = get_player_key(&$req) else {
            return build_response(Err(Errcode::NoPlayerKey));
        };
        let index = $srv.player_index.read().unwrap();
        let Some(id) = index.get(&key) else {
            return build_response(Err(Errcode::NoPlayerWithKey));
        };
        let players = $srv.players.read().unwrap();
        let player = players.get(id).unwrap();
        player.clone()
    }};
}

fn get_player_key(req: &HttpRequest) -> Option<PlayerKey> {
    for q in req.query_string().split("&") {
        if q.starts_with("key=") {
            let key = q.split("=").nth(1)?;
            let deckey = urlencoding::decode(key).ok()?;
            let mut key = [0; 128];
            BASE64_STANDARD
                .decode_slice(deckey.as_ref(), &mut key)
                .ok()?;
            return Some(key);
        }
    }
    None
}

pub fn jsonmerge(a: &mut Value, b: &Value) {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in b {
                jsonmerge(a.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (a, b) => *a = b.clone(),
    }
}

fn build_response(res: ApiResult) -> HttpResponse {
    let body = match res {
        Ok(mut data) => {
            jsonmerge(&mut data, &serde_json::json!({"error": "ok"}));
            data
        }
        Err(e) => {
            serde_json::json!({"error": e.errmsg(), "type": format!("{e:?}")})
        }
    };

    HttpResponse::Ok()
        .content_type("application/json")
        .json(&body)
}

#[web::get("/ping")]
async fn ping() -> impl web::Responder {
    build_response(Ok(json!({"ping": "pong"})))
}

#[web::post("/newplayer")]
async fn new_player(srv: GameState, req: Json<ReqNewPlayer>) -> impl web::Responder {
    let req = req.0;
    build_response(crate::player::new_player(srv, req))
}

#[web::get("/player/{id}")]
async fn get_player(srv: GameState, id: Path<u64>, req: HttpRequest) -> impl web::Responder {
    let Some(key) = get_player_key(&req) else {
        return build_response(Err(Errcode::NoPlayerKey));
    };

    let id = id.as_ref();
    build_response(crate::player::get_player(srv, *id, key))
}

#[web::get("/shipyard/list")]
async fn list_shipyard_ships(srv: GameState, req: HttpRequest) -> impl web::Responder {
    let player = get_player!(srv, req);
    let station_coord = player.read().unwrap().station;
    let station = srv.galaxy.get_station(station_coord).unwrap();
    build_response(crate::galaxy::station::list_shipyard_ships(station))
}

#[web::get("/shipyard/buy/{id}")]
async fn shipyard_buy(
    srv: GameState,
    id: Path<crate::ship::ShipId>,
    req: HttpRequest,
) -> impl web::Responder {
    let player = get_player!(srv, req);
    let station_coord = player.read().unwrap().station;
    let station = srv.galaxy.get_station(station_coord).unwrap();
    build_response(crate::galaxy::station::buy_ship(player, station, *id))
}

#[web::get("/crew/hire/{crewtype}")]
async fn hire_crew(
    srv: GameState,
    crewtype: Path<String>,
    req: HttpRequest,
) -> impl web::Responder {
    use crate::crew::CrewType;
    let player = get_player!(srv, req);
    let station_coord = player.read().unwrap().station;
    let station = srv.galaxy.get_station(station_coord).unwrap();
    let crewtype = match crewtype.as_str() {
        "pilot" => CrewType::Pilot,
        "operator" => CrewType::Operator,
        "trader" => CrewType::Trader,
        "soldier" => CrewType::Soldier,
        _ => return build_response(Err(Errcode::InvalidArgument("crewtype"))),
    };
    build_response(crate::crew::hire_crew(player, station, crewtype))
}

pub fn configure(srv: &mut ServiceConfig) {
    srv.service(ping)
        .service(hire_crew)
        .service(list_shipyard_ships)
        .service(shipyard_buy)
        .service(get_player)
        .service(new_player);
}
