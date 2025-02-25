use rand::Rng;
use rand_distr::{Distribution, Normal};
use serde::Serialize;
use std::collections::BTreeMap;
use strum::IntoEnumIterator;

use crate::{crew::CrewMember, ship::resources::Resource};

const MAX_AVG_AMPL: f64 = 5.0 / 100.0;
pub const MARKET_CHANGE_SEC: f64 = 20.0;
const BASE_FEE_RATE: f64 = 10.0 / 100.0;

// Buying 10000 worth of a resource can increase the price between 15% and 20%
const PRICE_INC_DIV: f64 = 10000.0;
const PRICE_INC_RANGE_MAX: f64 = 20.0 / 100.0;
const PRICE_INC_MIN_RATIO: f64 = 75.0 / 100.0;

#[inline]
pub fn fee_rate(rank: u8) -> f64 {
    BASE_FEE_RATE / (rank as f64)
}

// TODO (#24) Get from configuration
#[inline]
fn base_price(r: &Resource) -> f64 {
    match r {
        Resource::Stone => 1.0,
        Resource::Iron => 6.0,

        Resource::Helium => 1.0,
        Resource::Ozone => 6.0,
    }
}

#[derive(Serialize)]
pub struct Market {
    pub prices: BTreeMap<Resource, f64>,
}

impl Market {
    pub fn init() -> Market {
        let mut prices = BTreeMap::new();
        for r in Resource::iter() {
            prices.insert(r, base_price(&r));
        }
        Market { prices }
    }

    fn rand_distrib(&self, r: &Resource, now_price: f64) -> Normal<f64> {
        let base_price = base_price(r);
        let pratio = now_price / base_price;
        let avg = (1.0 - pratio) * MAX_AVG_AMPL;
        let std = avg.abs() + MAX_AVG_AMPL;

        rand_distr::Normal::new(avg, std).unwrap()
    }

    fn get_new_price<R: Rng>(&self, rng: &mut R, r: &Resource, old: f64) -> f64 {
        let distr = self.rand_distrib(r, old);
        let change = distr.sample(rng);
        old * (1.0 + change)
    }

    pub fn update_prices<R: Rng>(&mut self, rng: &mut R) {
        let new_prices = self
            .prices
            .iter()
            .map(|(r, p)| (*r, self.get_new_price(rng, r, *p)))
            .collect::<Vec<(Resource, f64)>>();

        for (r, price) in new_prices {
            let p = self.prices.get_mut(&r).unwrap();
            log::debug!("{r:?} {price} ({:?}%)", (price / base_price(&r)) * 100.0);
            *p = price;
        }
    }

    pub fn buy(&mut self, trader: &CrewMember, r: &Resource, amnt: f64) -> MarketTx {
        let fee_rate = fee_rate(trader.rank);
        let amnt_wfee = amnt * (1.0 - fee_rate);

        let price = *self.prices.get(r).unwrap();
        let price_inc_max = ((amnt * price) / PRICE_INC_DIV) * PRICE_INC_RANGE_MAX;
        let price_inc_min = price_inc_max * PRICE_INC_MIN_RATIO;
        let mut rng = rand::rng();
        let inc = rng.random_range(price_inc_min..price_inc_max);
        *self.prices.get_mut(r).unwrap() *= 1.0 + inc;

        let fees = (amnt * fee_rate) * price;
        MarketTx {
            added_cargo: Some((*r, amnt_wfee)),
            removed_money: Some(amnt * price),
            fees,
            ..Default::default()
        }
    }

    pub fn sell(&mut self, trader: &CrewMember, r: &Resource, amnt: f64) -> MarketTx {
        let fee_rate = fee_rate(trader.rank);

        let price = *self.prices.get(r).unwrap();
        let cost = amnt * price;
        let cost_wfee = cost * (1.0 - fee_rate);
        let fees = cost * fee_rate;

        let price_dec_max = (cost / PRICE_INC_DIV) * PRICE_INC_RANGE_MAX;
        let price_dec_min = price_dec_max * PRICE_INC_MIN_RATIO;
        let mut rng = rand::rng();
        let dec = rng.random_range(price_dec_min..price_dec_max);
        *self.prices.get_mut(r).unwrap() *= 1.0 - dec;

        MarketTx {
            removed_cargo: Some((*r, amnt)),
            added_money: Some(cost_wfee),
            fees,
            ..Default::default()
        }
    }
}

#[derive(Serialize, Default)]
pub struct MarketTx {
    pub added_cargo: Option<(Resource, f64)>,
    pub removed_cargo: Option<(Resource, f64)>,

    pub added_money: Option<f64>,
    pub removed_money: Option<f64>,
    pub fees: f64,
}
