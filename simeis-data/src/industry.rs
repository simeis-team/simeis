use std::collections::BTreeMap;

use rand::RngExt;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, EnumString, IntoStaticStr};

use crate::{crew::{CrewId, CrewMember, CrewMemberType}, ship::resources::Resource};

pub type IndustryUnitId = u32;

const UNIT_UPG_POWF_DIV: f64 = 75.0;

#[derive(
    EnumIter,
    EnumString,
    IntoStaticStr,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[strum(ascii_case_insensitive)]
pub enum IndustryUnitType {
    FuelRefinery,
    HullFoundry,
}

impl IndustryUnitType {
    pub fn new_unit(self) -> IndustryUnit {
        let unitid = rand::rng().random();
        IndustryUnit {
            id: unitid,
            operator: None,
            unittype: self,
            rank: 1,
            resources_required: vec![],
            resources_created: vec![],
        }
    }

    #[inline]
    pub fn get_price_buy(&self) -> f64 {
        8000.0
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IndustryUnit {
    pub id: IndustryUnitId,
    pub unittype: IndustryUnitType,
    pub rank: u8,

    operator: Option<CrewId>,
    resources_required: Vec<(Resource, f64)>,
    resources_created: Vec<(Resource, f64)>,
}

impl IndustryUnit {
    #[inline]
    pub fn price_next_rank(&self) -> f64 {
        let num = UNIT_UPG_POWF_DIV - 1.0 + (self.rank as f64);
        self.unittype.get_price_buy().powf(num / UNIT_UPG_POWF_DIV)
    }

    pub fn need_crew_member(&self, ctype: &CrewMemberType) -> bool {
        ctype == &CrewMemberType::Operator && self.operator.is_none()
    }

    pub fn assign_operator(&mut self, opid: CrewId, op: &CrewMember) {
        self.operator = Some(opid);
        self.new_op_rank(op.rank);
    }

    pub fn new_op_rank(&mut self, rank: u8) {
        self.resources_required = self.input(rank);
        self.resources_created = self.output(rank);
    }

    #[inline]
    pub fn input(&self, oprank: u8) -> Vec<(Resource, f64)> {
        let div = 1.0 / ((oprank as f64) + 1.0);
        match self.unittype {
            IndustryUnitType::FuelRefinery => vec![
                (Resource::Carbon, 1.9),
                (Resource::Hydrogen, 1.2),
                (Resource::Oxygen, 0.3),
                (Resource::Water, 0.5),
            ],
            IndustryUnitType::HullFoundry => vec![
                (Resource::Carbon, 0.5),
                (Resource::Iron, 0.7),
                (Resource::Hydrogen, 0.3),
                (Resource::Water, 0.5),
            ],
        }
        .into_iter()
        .map(|(res, amnt)| {
            let amnt : f64 = amnt;
            let new_amnt = amnt.powf(div);
            (res, new_amnt)
        }).collect()
    }

    #[inline]
    pub fn output(&self, oprank: u8) -> Vec<(Resource, f64)> {
        let pown = ((oprank as f64) + 1.0).ln();
        match self.unittype {
            IndustryUnitType::FuelRefinery => vec![(Resource::Fuel, 1.0)],
            IndustryUnitType::HullFoundry => vec![(Resource::Hull, 1.0)],
        }
        .into_iter()
        .map(|(res, amnt)| {
            let amnt: f64 = amnt;
            (res, amnt.powf(pown))
        })
        .collect()
    }

    pub fn can_work(&self, tdelta: &f64, resources: &BTreeMap<Resource, f64>) -> bool {
        if self.operator.is_none() {
            return false;
        }
        self.resources_required
            .iter()
            .all(|(res, amnt)| {
                if let Some(incargo) = resources.get(res) {
                    incargo >= &(amnt * tdelta)
                } else {
                    false
                }
            })
    }

    pub fn work(&self, tdelta: &f64, resources: &mut BTreeMap<Resource, f64>) {
        for (res, amnt) in self.resources_required.iter() {
            let n = resources.get_mut(res).unwrap();
            *n -= amnt * tdelta;
        }

        for (res, amnt) in self.resources_created.iter() {
            let n = resources.get_mut(res).unwrap();
            *n += amnt * tdelta;
        }
    }
}

