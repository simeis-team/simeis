use std::collections::{BTreeMap, HashMap};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;

use crate::galaxy::Galaxy;
use crate::player::{Player, PlayerId, PlayerKey};

const ITER_PERIOD: std::time::Duration = std::time::Duration::from_millis(100);

// TODO (#23) Have a global "inflation" rate for all users, that increases over time
//     Equipment becomes more and more expansive

#[derive(Clone)]
pub struct Game {
    pub players: Arc<RwLock<BTreeMap<PlayerId, Arc<RwLock<Player>>>>>,
    pub player_index: Arc<RwLock<HashMap<PlayerKey, PlayerId>>>,
    pub galaxy: Galaxy,
    send_stop: Sender<bool>,
}

impl Game {
    pub fn init() -> (JoinHandle<()>, Game) {
        let (send_stop, recv_stop) = std::sync::mpsc::channel();
        let data = Game {
            players: Arc::new(RwLock::new(BTreeMap::new())),
            galaxy: Galaxy::init(),
            player_index: Arc::new(RwLock::new(HashMap::new())),
            send_stop,
        };
        let thread_data = data.clone();

        let thread = std::thread::spawn(move || thread_data.start(recv_stop));
        (thread, data)
    }

    pub fn start(&self, stop: Receiver<bool>) {
        log::debug!("Started thread");
        let sleepmin_iter = ITER_PERIOD;
        let mut last_iter = std::time::Instant::now();
        while stop.try_recv().is_err_and(|x| x == TryRecvError::Empty) {
            self.threadloop();
            let took = std::time::Instant::now() - last_iter;
            std::thread::sleep(sleepmin_iter - took);
            last_iter = std::time::Instant::now();
        }
        log::info!("Exiting game thread");
    }

    fn threadloop(&self) {
        for (_, player) in self.players.read().unwrap().iter() {
            let mut player = player.write().unwrap();
            player.update_money(ITER_PERIOD.as_secs_f64());
            let mut deadship = vec![];
            for (id, ship) in player.ships.iter_mut() {
                let finished = ship.update_flight(ITER_PERIOD.as_secs_f64());
                if finished {
                    ship.state = crate::ship::ShipState::Idle;
                    if ship.hull_decay >= ship.hull_decay_capacity {
                        deadship.push(*id);
                    }
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
