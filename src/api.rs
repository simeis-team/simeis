use std::collections::HashMap;
use std::ops::DerefMut;

use base64::{prelude::BASE64_STANDARD, Engine};
use ntex::web::types::{Json, Path};
use ntex::web::{self, HttpRequest, HttpResponse, ServiceConfig};
use serde_json::{json, Value};
use strum::IntoEnumIterator;

use crate::crew::{CrewId, CrewMemberType};
use crate::errors::Errcode;
use crate::galaxy::station::StationId;
use crate::player::{PlayerId, PlayerKey, ReqNewPlayer};
use crate::ship::module::{ShipModuleId, ShipModuleType};
use crate::ship::navigation::Travel;
use crate::ship::resources::Resource;
use crate::ship::ShipId;
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

macro_rules! get_station {
    ($srv:ident, $player:ident, $id:expr) => {{
        let player = $player.read().unwrap();
        let Some(station_coord) = player.stations.get($id) else {
            return build_response(Err(Errcode::NoSuchStation(*$id)));
        };
        $srv.galaxy.get_station(station_coord).unwrap()
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

#[web::post("/player/new")]
async fn new_player(srv: GameState, req: Json<ReqNewPlayer>) -> impl web::Responder {
    let req = req.0;
    build_response(crate::player::new_player(srv, req))
}

#[web::get("/player/{id}")]
async fn get_player(srv: GameState, id: Path<PlayerId>, req: HttpRequest) -> impl web::Responder {
    let Some(key) = get_player_key(&req) else {
        return build_response(Err(Errcode::NoPlayerKey));
    };

    let id = id.as_ref();
    build_response(crate::player::get_player(srv, *id, key))
}

#[web::get("/station/{station_id}/shipyard/list")]
async fn list_shipyard_ships(
    srv: GameState,
    id: Path<StationId>,
    req: HttpRequest,
) -> impl web::Responder {
    let player = get_player!(srv, req);
    let station = get_station!(srv, player, id.as_ref());
    build_response(crate::galaxy::station::list_shipyard_ships(station))
}

#[web::get("/station/{station_id}/shipyard/buy/{id}")]
async fn shipyard_buy(
    srv: GameState,
    args: Path<(StationId, ShipId)>,
    req: HttpRequest,
) -> impl web::Responder {
    let (station_id, ship_id) = args.as_ref();
    let player = get_player!(srv, req);
    let station = get_station!(srv, player, station_id);
    build_response(crate::galaxy::station::buy_ship(player, station, *ship_id))
}

#[web::get("/station/{station_id}/crew/idle")]
async fn idle_crew(srv: GameState, id: Path<StationId>, req: HttpRequest) -> impl web::Responder {
    let player = get_player!(srv, req);
    let station = get_station!(srv, player, id.as_ref());
    build_response(crate::galaxy::station::get_idle_crew(station))
}

#[web::get("/station/{station_id}/crew/hire/{crewtype}")]
async fn hire_crew(
    srv: GameState,
    args: Path<(StationId, String)>,
    req: HttpRequest,
) -> impl web::Responder {
    let (station_id, crewtype) = args.as_ref();
    let Some(crewtype) = CrewMemberType::from_str(crewtype.as_str()) else {
        return build_response(Err(Errcode::InvalidArgument("crewtype")));
    };
    let player = get_player!(srv, req);
    let station = get_station!(srv, player, station_id);
    build_response(crate::crew::hire_crew(player, station, crewtype))
}

#[web::get("/station/{station_id}/crew/assign/{crewid}/{shipid}/{modid}")]
async fn assign_crew(
    args: Path<(StationId, CrewId, ShipId, ShipModuleId)>,
    srv: GameState,
    req: HttpRequest,
) -> impl web::Responder {
    let (station_id, crew_id, ship_id, modid) = args.as_ref();
    let player = get_player!(srv, req);
    let station = get_station!(srv, player, station_id);
    let mut player = player.write().unwrap();
    let Some(ship) = player.ships.get_mut(ship_id) else {
        return build_response(Err(Errcode::ShipNotFound(*ship_id)));
    };
    build_response(crate::crew::assign_crew_member(
        *crew_id, station, ship, modid,
    ))
}

#[web::get("/station/{station_id}/scan")]
async fn scan(id: Path<StationId>, srv: GameState, req: HttpRequest) -> impl web::Responder {
    let player = get_player!(srv, req);
    let station = get_station!(srv, player, id.as_ref());
    let results = station.read().unwrap().scan(&srv.galaxy);
    build_response(Ok(results.to_json()))
}

#[web::get("/station/{station_id}/shop/modules/{ship_id}/buy/{modtype}")]
async fn buy_ship_module(
    srv: GameState,
    args: Path<(StationId, ShipId, String)>,
    req: HttpRequest,
) -> impl web::Responder {
    let (station_id, ship_id, modtype) = args.as_ref();
    let player = get_player!(srv, req);

    let Some(modtype) = ShipModuleType::from_str(modtype.as_str()) else {
        return build_response(Err(Errcode::InvalidArgument("modtype")));
    };
    let mut player = player.write().unwrap();
    build_response(
        player
            .buy_ship_module(station_id, ship_id, modtype)
            .map(|v| {
                serde_json::json!({
                    "id": v,
                })
            }),
    )
}

#[web::get("/station/{station_id}/shop/cargo/buy/{amount}")]
async fn buy_station_cargo(
    srv: GameState,
    args: Path<(StationId, usize)>,
    req: HttpRequest,
) -> impl web::Responder {
    let (id, amnt) = args.as_ref();
    let player = get_player!(srv, req);
    let station = get_station!(srv, player, id);

    let mut player = player.write().unwrap();
    let mut station = station.write().unwrap();
    build_response(
        station
            .buy_cargo(player.deref_mut(), amnt)
            .map(|v| serde_json::to_value(v).unwrap()),
    )
}

#[web::get("/station/{station_id}/shop/cargo/price")]
async fn get_station_cargo_price(
    srv: GameState,
    id: Path<StationId>,
    req: HttpRequest,
) -> impl web::Responder {
    let player = get_player!(srv, req);
    let station = get_station!(srv, player, id.as_ref());
    let price = station.read().unwrap().cargo_price();
    build_response(Ok(serde_json::json!({
        "price": price,
    })))
}

#[web::get("/station/{station_id}/cargo")]
async fn get_station_cargo(
    srv: GameState,
    id: Path<StationId>,
    req: HttpRequest,
) -> impl web::Responder {
    let player = get_player!(srv, req);
    let station = get_station!(srv, player, id.as_ref());
    let station = station.read().unwrap();
    build_response(Ok(serde_json::to_value(&station.cargo).unwrap()))
}

#[web::get("/ship/{ship_id}")]
async fn get_ship_status(
    srv: GameState,
    id: Path<ShipId>,
    req: HttpRequest,
) -> impl web::Responder {
    let player = get_player!(srv, req);
    let player = player.read().unwrap();
    let Some(ship) = player.ships.get(id.as_ref()) else {
        return build_response(Err(Errcode::ShipNotFound(*id)));
    };
    build_response(Ok(serde_json::to_value(ship).unwrap()))
}

#[web::post("/ship/{ship_id}/travelcost")]
async fn compute_travel_costs(
    srv: GameState,
    id: Path<ShipId>,
    travel: Json<Travel>,
    req: HttpRequest,
) -> impl web::Responder {
    let player = get_player!(srv, req);
    let player = player.read().unwrap();
    let Some(ship) = player.ships.get(id.as_ref()) else {
        return build_response(Err(Errcode::ShipNotFound(*id)));
    };
    build_response(travel.0.compute_costs(ship).map(|v| serde_json::json!(v)))
}

#[web::post("/ship/{ship_id}/navigate")]
async fn ask_navigate(
    srv: GameState,
    id: Path<ShipId>,
    travel: Json<Travel>,
    req: HttpRequest,
) -> impl web::Responder {
    let player = get_player!(srv, req);
    let mut player = player.write().unwrap();
    let Some(ship) = player.ships.get_mut(id.as_ref()) else {
        return build_response(Err(Errcode::ShipNotFound(*id)));
    };
    build_response(
        ship.set_travel(travel.0)
            .map(|cost| serde_json::json!(cost)),
    )
}

#[web::get("/ship/{ship_id}/extraction/start")]
async fn start_extraction(
    srv: GameState,
    id: Path<ShipId>,
    req: HttpRequest,
) -> impl web::Responder {
    let player = get_player!(srv, req);
    let mut player = player.write().unwrap();
    let Some(ship) = player.ships.get_mut(id.as_ref()) else {
        return build_response(Err(Errcode::ShipNotFound(*id)));
    };
    build_response(
        ship.start_extraction(&srv.galaxy)
            .map(|v| serde_json::to_value(v).unwrap()),
    )
}

#[web::get("/ship/{ship_id}/extraction/stop")]
async fn stop_extraction(
    srv: GameState,
    id: Path<ShipId>,
    req: HttpRequest,
) -> impl web::Responder {
    let player = get_player!(srv, req);
    let mut player = player.write().unwrap();
    let Some(ship) = player.ships.get_mut(id.as_ref()) else {
        return build_response(Err(Errcode::ShipNotFound(*id)));
    };
    build_response(
        ship.stop_extraction()
            .map(|v| serde_json::to_value(v).unwrap()),
    )
}

#[web::get("/ship/{ship_id}/unload/{resource}/{amount}")]
async fn unload_ship_cargo(
    srv: GameState,
    args: Path<(ShipId, String, f64)>,
    req: HttpRequest,
) -> impl web::Responder {
    let (id, resource, amnt) = args.as_ref();
    let Some(resource) = Resource::from_str(resource) else {
        return build_response(Err(Errcode::InvalidArgument("resource")));
    };

    let player = get_player!(srv, req);
    let mut player = player.write().unwrap();

    let Some(ship) = player.ships.get(id) else {
        return build_response(Err(Errcode::ShipNotFound(*id)));
    };

    let Some(station) = player.stations.iter().find(|(_, s)| *s == &ship.position) else {
        return build_response(Err(Errcode::ShipNotInStation));
    };

    let station = srv.galaxy.get_station(station.1).unwrap();
    let mut station = station.write().unwrap();
    let ship = player.ships.get_mut(id).unwrap();
    build_response(
        ship.unload_cargo(&resource, *amnt, station.deref_mut())
            .map(|v| serde_json::json!({ "unloaded": v })),
    )
}

#[web::get("/prices/ship_module")]
async fn get_prices_ship_module() -> impl web::Responder {
    let mut res: HashMap<String, f64> = HashMap::new();
    for smod in ShipModuleType::iter() {
        res.insert(format!("{smod:?}"), smod.get_price_buy());
    }
    build_response(Ok(serde_json::to_value(res).unwrap()))
}

pub fn configure(srv: &mut ServiceConfig) {
    srv.service(ping)
        .service(hire_crew)
        .service(idle_crew)
        .service(assign_crew)
        .service(get_ship_status)
        .service(shipyard_buy)
        .service(get_prices_ship_module)
        .service(buy_ship_module)
        .service(list_shipyard_ships)
        .service(ask_navigate)
        .service(compute_travel_costs)
        .service(start_extraction)
        .service(stop_extraction)
        .service(unload_ship_cargo)
        .service(get_station_cargo)
        .service(get_station_cargo_price)
        .service(buy_station_cargo)
        .service(scan)
        .service(get_player)
        .service(new_player);
}
