use ntex::web::{self, HttpRequest, HttpResponse};

mod errors;
mod galaxy;
mod game;
mod player;
#[cfg(test)]
mod tests;

pub type ServerState = ntex::web::types::State<game::Game>;

pub fn build_response(data: serde_json::Value) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .json(&data)
}

pub fn get_json_key(data: &serde_json::Value, key: &'static str) -> Option<serde_json::Value> {
    let keys = key.split(".").collect::<Vec<&'static str>>();
    let serde_json::Value::Object(map) = data else {
        return None;
    };

    let key_tot = keys.len();
    let mut data = map;
    for (nk, key) in keys.into_iter().enumerate() {
        if nk == (key_tot - 1) {
            return data.get(key).cloned();
        } else {
            let inner = data.get(key)?.as_object()?;
            data = inner;
        }
    }
    unreachable!()
}

pub fn get_player_key(req: &HttpRequest) -> Option<std::borrow::Cow<'_, str>> {
    for q in req.query_string().split("&") {
        if q.starts_with("key=") {
            let key = q.split("=").nth(1)?;
            let deckey = urlencoding::decode(key).ok()?;
            return Some(deckey);
        }
    }
    None
}

#[web::get("/ping")]
async fn ping() -> impl web::Responder {
    build_response(serde_json::json!({ "error": "ok", "ping": "pong"}))
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::builder()
        .filter_module("ntex_server", log::LevelFilter::Warn)
        .init();
    log::info!("Running on http://127.0.0.1:8080");
    let (gamethread, state) = game::Game::init();
    let game = state.clone();

    #[allow(clippy::redundant_closure)] // DEV
    let res = web::HttpServer::new(move || {
        web::App::new()
            .wrap(web::middleware::Logger::default())
            .state(state.clone())
            .service(ping)
            .configure(|srv| player::configure(srv))
    })
    .stop_runtime()
    .bind(("127.0.0.1", 8080))?
    .run()
    .await;

    game.stop(gamethread);
    res
}
