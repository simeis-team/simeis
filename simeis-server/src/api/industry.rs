use std::collections::BTreeMap;
use std::str::FromStr;

use ntex::router::IntoPattern;
use ntex::web;
use ntex::web::scope;
use ntex::web::types::Path;
use ntex::web::HttpRequest;
use ntex::web::ServiceConfig;

use serde_json::json;
use serde_json::to_value;

use simeis_data::errors::Errcode;
use simeis_data::galaxy::station::StationId;
use simeis_data::industry::IndustryUnitId;
use simeis_data::industry::IndustryUnitType;
use strum::IntoEnumIterator;

use crate::api::build_response;
use crate::api::GameState;

// Buy a new industry unit
#[web::get("/buy/list")]
async fn list_buy_industry(
    args: Path<StationId>,
    srv: GameState,
    req: HttpRequest,
) -> impl web::Responder {
    let pkey = get_player_key!(req);
    let station_id = *args;

    let data = srv
        .map_player(&pkey, |player| {
            Box::pin(async move {
                let Some(_station) = player.stations.get(&station_id).cloned() else {
                    return Err(Errcode::NoSuchStation(station_id));
                };
                let mut res: BTreeMap<IndustryUnitType, f64> = BTreeMap::new();
                for unit in IndustryUnitType::iter() {
                    let price = unit.get_price_buy();
                    res.insert(unit, price);
                }
                Ok(to_value(res).unwrap())
            })
        })
        .await;
    build_response(data)
}

// Buy a new industry unit
#[web::post("/buy/{name}")]
async fn buy_industry(
    args: Path<(StationId, String)>,
    srv: GameState,
    req: HttpRequest,
) -> impl web::Responder {
    let pkey = get_player_key!(req);
    let (station_id, indutype) = args.clone();
    let Ok(indutype) = IndustryUnitType::from_str(indutype.as_str()) else {
        return build_response(Err(Errcode::InvalidArgument("industry_type")));
    };

    let data = srv
        .map_player_mut(&pkey, |player| {
            Box::pin(async move {
                let Some(station) = player.stations.get(&station_id).cloned() else {
                    return Err(Errcode::NoSuchStation(station_id));
                };
                let (id, cost) = station.buy_industry(player, indutype).await?;
                Ok(json!({ "id": id, "cost": cost }))
            })
        })
        .await;
    build_response(data)
}

// Upgrade an industry unit
#[web::post("/upgrade/{id}")]
async fn upgrade_industry(
    args: Path<(StationId, IndustryUnitId)>,
    srv: GameState,
    req: HttpRequest,
) -> impl web::Responder {
    let pkey = get_player_key!(req);
    let (station_id, id) = *args;

    let data = srv
        .map_player_mut(&pkey, |player| {
            Box::pin(async move {
                let Some(station) = player.stations.get(&station_id).cloned() else {
                    return Err(Errcode::NoSuchStation(station_id));
                };
                let newrank = station.upgrade_industry(player, &id).await?;
                Ok(json!({ "new-rank": newrank }))
            })
        })
        .await;
    build_response(data)
}

// Upgrade an industry unit
#[web::post("/start/{id}")]
async fn start_industry(
    args: Path<(StationId, IndustryUnitId)>,
    srv: GameState,
    req: HttpRequest,
) -> impl web::Responder {
    let pkey = get_player_key!(req);
    let (station_id, id) = *args;

    let data = srv
        .map_station(&pkey, &station_id, |pid, station| {
            Box::pin(async move {
                station.start_industry(pid, &id).await?;
                Ok(to_value(()).unwrap())
            })
        })
        .await;
    build_response(data)
}

// Upgrade an industry unit
#[web::post("/stop/{id}")]
async fn stop_industry(
    args: Path<(StationId, IndustryUnitId)>,
    srv: GameState,
    req: HttpRequest,
) -> impl web::Responder {
    let pkey = get_player_key!(req);
    let (station_id, id) = *args;

    let data = srv
        .map_station(&pkey, &station_id, |pid, station| {
            Box::pin(async move {
                station.stop_industry(pid, &id).await?;
                Ok(to_value(()).unwrap())
            })
        })
        .await;
    build_response(data)
}

// Shows the production inputs & outputs of a particular unit
#[web::get("/production/{id}")]
async fn show_production(
    args: Path<(StationId, IndustryUnitId)>,
    srv: GameState,
    req: HttpRequest,
) -> impl web::Responder {
    let pkey = get_player_key!(req);
    let (station_id, id) = *args;

    let data = srv.map_station(&pkey, &station_id, |pid, station| {
        Box::pin(async move {
            let (inputs, outputs) = station.get_industry_production(pid, id).await?;
            Ok(json!({
                "inputs": to_value(inputs).unwrap(),
                "outputs": to_value(outputs).unwrap(),
            }))
        })
    }).await;

    build_response(data)
}

pub fn configure<T: IntoPattern>(base: T, srv: &mut ServiceConfig) {
    srv.service(
        scope(base)
            .service(list_buy_industry)
            .service(buy_industry)
            .service(upgrade_industry)
            .service(show_production)
            .service(start_industry)
            .service(stop_industry)
    );
}
