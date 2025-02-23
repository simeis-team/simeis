use ntex::web;

mod api;
mod crew;
mod errors;
mod galaxy;
mod game;
mod player;
mod ship;
#[cfg(test)]
mod tests;

pub type GameState = ntex::web::types::State<game::Game>;

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

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::builder()
        .filter_module("ntex_server", log::LevelFilter::Warn)
        .filter_module("ntex_io", log::LevelFilter::Warn)
        .init();
    log::info!("Running on http://127.0.0.1:8080");
    let (gamethread, state) = game::Game::init();
    let game = state.clone();

    #[allow(clippy::redundant_closure)] // DEV
    let res = web::HttpServer::new(move || {
        web::App::new()
            .wrap(web::middleware::Logger::default())
            .state(state.clone())
            .configure(|srv| api::configure(srv))
    })
    .stop_runtime()
    .bind(("127.0.0.1", 8080))?
    .run()
    .await;

    game.stop(gamethread);
    res
}
