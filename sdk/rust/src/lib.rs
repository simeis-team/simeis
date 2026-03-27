#![allow(dead_code, unused_variables)]
use serde_json::{Map, Value};
use std::{collections::HashMap, path::PathBuf, time::Duration};

pub fn get_id(data: &Value) -> u64 {
    json_get_uint("id", data).unwrap()
}

pub fn get_planet_position(planet: &Value) -> Option<(u64, u64, u64)> {
    let v = json_get_list("position", planet)?;
    match (
        v.first().and_then(|n| n.as_u64()),
        v.get(1).and_then(|n| n.as_u64()),
        v.get(2).and_then(|n| n.as_u64()),
    ) {
        (Some(x), Some(y), Some(z)) => Some((x, y, z)),
        _ => None,
    }
}

pub fn json_get_key<'a>(key: &str, mut val: &'a Value) -> Option<&'a Value> {
    for k in key.split(".") {
        let obj = val.as_object()?;
        val = obj.get(k)?;
    }
    Some(val)
}
pub fn json_get_list<'a>(key: &str, val: &'a Value) -> Option<&'a Vec<Value>> {
    json_get_key(key, val)?.as_array()
}
pub fn json_get_dict<'a>(key: &str, val: &'a Value) -> Option<&'a Map<String, Value>> {
    json_get_key(key, val)?.as_object()
}
pub fn json_get_float(key: &str, val: &Value) -> Option<f64> {
    json_get_key(key, val)?.as_f64()
}
pub fn json_get_string<'a>(key: &str, val: &'a Value) -> Option<&'a str> {
    json_get_key(key, val)?.as_str()
}
pub fn json_get_uint(key: &str, val: &Value) -> Option<u64> {
    json_get_key(key, val)?.as_u64()
}
pub fn json_get_bool(key: &str, val: &Value) -> Option<bool> {
    json_get_key(key, val)?.as_bool()
}

pub type ApiResult = Result<Value, Value>;

pub struct SimeisSDK {
    url: String,
    player_id: Option<u64>,
    player_key: Option<String>,
}

impl SimeisSDK {
    pub fn new<T: ToString>(username: String, ip: T, port: u16) -> SimeisSDK {
        let url = format!("http://{}:{port}", ip.to_string());
        let mut sdk = SimeisSDK {
            url,
            player_id: None,
            player_key: None,
        };
        assert!(sdk.ping());
        sdk.setup_player(username, false);
        sdk
    }

    pub fn get<T: ToString>(&self, path: T) -> ApiResult {
        debug_assert!(path.to_string().starts_with("/"));
        println!("GET {}", path.to_string());

        let mut req = ureq::get(format!("{}{}", self.url, path.to_string()));
        if let Some(ref key) = self.player_key {
            req = req.query("key", key);
        }
        let mut got = req.call().map_err(|err| {
            serde_json::json!({
                "status": 0,
                "error": format!("{err:?}"),
            })
        })?;
        let data = got
            .body_mut()
            .read_to_string()
            .expect("Non textual data from server");
        let mut data: Value = serde_json::from_str(&data)
            .expect("Unable to decode JSON from what the server returned");
        let datamap = data.as_object_mut().unwrap();
        let err = datamap
            .remove("error")
            .expect("Missing error field in reply");
        if err != "ok" { Err(data) } else { Ok(data) }
    }

    pub fn ping(&self) -> bool {
        let Ok(got) = self.get("/ping") else {
            return false;
        };
        matches!(json_get_string("ping", &got), Some("pong"))
    }

    fn setup_player(&mut self, mut username: String, force_register: bool) {
        username = username
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_lowercase();

        let path = PathBuf::from(format!("./{username}.json"));

        let player = if !path.exists() || force_register {
            println!("Creating player {username}");
            let player_tmp = self.get(format!("/player/new/{username}")).unwrap();
            let player_json_str = serde_json::to_string(&player_tmp).unwrap();
            println!("{player_tmp:?}");
            std::fs::write(&path, player_json_str).expect("Unable to write player data to path");
            player_tmp
        } else {
            let player_json_str =
                std::fs::read_to_string(&path).expect("Unable to read player data from path");
            serde_json::from_str(&player_json_str).expect("Unable to load player JSON data")
        };

        self.player_key = Some(json_get_string("key", &player).unwrap().to_string());
        self.player_id = Some(json_get_uint("playerId", &player).unwrap());

        let player_id = json_get_uint("playerId", &player).unwrap();
        std::thread::sleep(Duration::from_secs(1));
        println!("{:?}", self.get(format!("/player/{player_id}")));
        let Ok(player_status) = self.get(format!("/player/{player_id}")) else {
            return self.setup_player(username, true);
        };

        let player_money = json_get_float("money", &player_status).unwrap();
        if player_money <= 0.0 {
            println!("!!! Player already lost, please restart the server to reset the game");
            std::process::exit(0);
        }
    }

    pub fn get_player_status(&self) -> ApiResult {
        self.get(format!("/player/{}", self.player_id.unwrap()))
    }

    pub fn get_ship_status(&self, ship_id: u64) -> ApiResult {
        self.get(format!("/ship/{ship_id}"))
    }

    pub fn get_station_status(&self, station_id: u64) -> ApiResult {
        self.get(format!("/station/{station_id}"))
    }

    pub fn shop_list_modules(&self, station_id: u64) -> Result<Vec<Value>, Value> {
        let mut all = self.get(format!("/station/{station_id}/shop/modules"))?;
        let allvec = all.as_array_mut().unwrap();
        allvec.sort_by(|a, b| {
            let pa = json_get_float("price", a).unwrap();
            let pb = json_get_float("price", b).unwrap();
            pa.partial_cmp(&pb).unwrap()
        });
        let Value::Array(data) = all else {
            unreachable!();
        };
        Ok(data)
    }

    pub fn shop_list_ship(&self, station_id: u64) -> Result<Vec<Value>, Value> {
        let mut all = self.get(format!("/station/{station_id}/shipyard/list"))?;
        let allvec = all.as_array_mut().unwrap();
        allvec.sort_by(|a, b| {
            let pa = json_get_float("price", a).unwrap();
            let pb = json_get_float("price", b).unwrap();
            pa.partial_cmp(&pb).unwrap()
        });
        let Value::Array(data) = all else {
            unreachable!();
        };
        Ok(data)
    }

    pub fn buy_ship(&self, station_id: u64, ship_id: u64) -> ApiResult {
        self.get(format!("/station/{station_id}/shipyard/buy/{ship_id}"))
    }

    pub fn buy_module_on_ship(&self, station_id: u64, ship_id: u64, modtype: &str) -> ApiResult {
        self.get(format!(
            "/station/{station_id}/shop/modules/{ship_id}/buy/{}",
            modtype.to_lowercase()
        ))
    }

    pub fn hire_crew(&self, station_id: u64, crewtype: &str) -> ApiResult {
        self.get(format!(
            "/station/{station_id}/crew/hire/{}",
            crewtype.to_lowercase()
        ))
    }

    pub fn assign_crew_to_ship_module(
        &self,
        station_id: u64,
        ship_id: u64,
        crew_id: u64,
        mod_id: u64,
    ) -> ApiResult {
        self.get(format!(
            "/station/{station_id}/crew/assign/{crew_id}/{ship_id}/{mod_id}"
        ))
    }

    pub fn assign_crew_as_ship_pilot(
        &self,
        station_id: u64,
        ship_id: u64,
        pilot_id: u64,
    ) -> ApiResult {
        self.get(format!(
            "/station/{station_id}/crew/assign/{pilot_id}/{ship_id}/pilot"
        ))
    }

    pub fn station_has_trader(&self, station_id: u64) -> Result<bool, Value> {
        let station = self.get_station_status(station_id)?;
        for cm in json_get_list("crew", &station).unwrap() {
            if json_get_string("member_type", cm).unwrap() == "Trader" {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn assign_trader_to_station(&self, station_id: u64, crew_id: u64) -> ApiResult {
        self.get(format!(
            "/station/{station_id}/crew/assign/{crew_id}/trading"
        ))
    }

    pub fn compute_travel_cost(&self, ship_id: u64, position: (u64, u64, u64)) -> ApiResult {
        let (x, y, z) = position;
        self.get(format!("/ship/{ship_id}/travelcost/{x}/{y}/{z}"))
    }

    // TODO
    pub fn travel(&self, ship_id: u64, position: (u64, u64, u64), wait_end: bool) -> ApiResult {
        todo!()
    }

    pub fn wait_until_ship_idle(&self, ship_id: u64, time_sleep: Duration) -> ApiResult {
        todo!()
    }
    pub fn buy_plates_for_repair(&self, station_id: u64, ship_id: u64) -> ApiResult {
        todo!()
    }
    pub fn repair_ship(&self, station_id: u64, ship_id: u64) -> ApiResult {
        todo!()
    }
    pub fn buy_fuel_for_refuel(&self, station_id: u64, ship_id: u64) -> ApiResult {
        todo!()
    }
    pub fn refuel_ship(&self, station_id: u64, ship_id: u64) -> ApiResult {
        todo!()
    }
    pub fn scan_planets(&self, station_id: u64) -> Result<Vec<Value>, Value> {
        todo!()
    }
    pub fn mine(&self, ship_id: u64) -> ApiResult {
        todo!()
    }
    pub fn return_station_and_unload(&self, station_id: u64, ship_id: u64) -> ApiResult {
        todo!()
    }
    pub fn get_station_resources(&self, station_id: u64) -> Result<HashMap<String, f64>, Value> {
        todo!()
    }
    pub fn get_market_prices(&self) -> Result<HashMap<String, f64>, Value> {
        todo!()
    }
    pub fn sell_resource(&self, station_id: u64, resource: &str, amnt: f64) -> ApiResult {
        todo!()
    }
    pub fn buy_resource(&self, station_id: u64, resource: &str, amnt: f64) -> ApiResult {
        todo!()
    }
}
