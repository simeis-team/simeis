use std::collections::{BTreeMap, HashMap};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rand::Rng;

use crate::galaxy::Galaxy;
use crate::market::{Market, MARKET_CHANGE_SEC};
use crate::player::{Player, PlayerId, PlayerKey};
use crate::ship::ShipState;

const ITER_PERIOD: Duration = Duration::from_millis(105);

// TODO (#23) Have a global "inflation" rate for all users, that increases over time
//     Equipment becomes more and more expansive

#[derive(Clone)]
pub struct Game {
    pub players: Arc<RwLock<BTreeMap<PlayerId, Arc<RwLock<Player>>>>>,
    pub player_index: Arc<RwLock<HashMap<PlayerKey, PlayerId>>>,
    pub galaxy: Galaxy,
    pub market: Arc<RwLock<Market>>,
    send_stop: Sender<bool>,
}

impl Game {
    pub fn init() -> (JoinHandle<()>, Game) {
        let (send_stop, recv_stop) = std::sync::mpsc::channel();
        let data = Game {
            send_stop,
            galaxy: Galaxy::init(),
            market: Arc::new(RwLock::new(Market::init())),
            players: Arc::new(RwLock::new(BTreeMap::new())),
            player_index: Arc::new(RwLock::new(HashMap::new())),
        };
        let thread_data = data.clone();

        let thread = std::thread::spawn(move || thread_data.start(recv_stop));
        (thread, data)
    }

    pub fn start(&self, stop: Receiver<bool>) {
        log::debug!("Started thread");
        let sleepmin_iter = ITER_PERIOD;
        let mut last_iter = Instant::now();
        let mut market_last_tick = Instant::now();
        let mut rng = rand::rng();
        while stop.try_recv().is_err_and(|x| x == TryRecvError::Empty) {
            self.threadloop(&mut rng, &mut market_last_tick);
            let took = Instant::now() - last_iter;
            std::thread::sleep(sleepmin_iter - took);
            last_iter = Instant::now();
        }
        log::info!("Exiting game thread");
    }

    fn threadloop<R: Rng>(&self, rng: &mut R, mlt: &mut Instant) {
        let market_change_proba = (mlt.elapsed().as_secs_f64() / MARKET_CHANGE_SEC).min(1.0);
        if rng.random_bool(market_change_proba) {
            self.market.write().unwrap().update_prices(rng);
            *mlt = Instant::now();
        }
        for (_, player) in self.players.read().unwrap().iter() {
            let mut player = player.write().unwrap();
            player.update_money(ITER_PERIOD.as_secs_f64());
            let mut deadship = vec![];
            for (id, ship) in player.ships.iter_mut() {
                match ship.state {
                    ShipState::InFlight(..) => {
                        let finished = ship.update_flight(ITER_PERIOD.as_secs_f64());
                        if finished {
                            ship.state = ShipState::Idle;
                            if ship.hull_decay >= ship.hull_decay_capacity {
                                deadship.push(*id);
                            }
                        }
                    }

                    ShipState::Extracting(..) => {
                        let finished = ship.update_extract(ITER_PERIOD.as_secs_f64());
                        if finished {
                            ship.state = ShipState::Idle;
                        }
                    }
                    _ => {}
                }
            }
            for id in deadship {
                player.ships.remove(&id);
            }
        }
    }

    pub fn stop(self, handle: JoinHandle<()>) {
        log::info!("Asking game thread to exit");
        self.send_stop.send(true).unwrap();
        let _ = handle.join();
        log::info!("Game stopped");
    }
}
