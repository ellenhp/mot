pub mod base;
pub mod pedestrian;
pub mod units;

use std::{collections::HashMap, mem::ManuallyDrop, ops::Add};

use evmap::ShallowCopy;
use serde::{Deserialize, Serialize};
use units::{Direction, ElapsedTime, PartsPerMillion, TravelSpeed, TravelledDistance};

use crate::graph::{WayId, WayTransition};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoutingCost {
    cost_millis: ElapsedTime,
    actual_millis: ElapsedTime,
    distance_mm: TravelledDistance,
}

impl PartialOrd for RoutingCost {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.cost_millis.partial_cmp(&other.cost_millis) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.actual_millis.partial_cmp(&other.actual_millis) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.distance_mm.partial_cmp(&other.distance_mm)
    }
}

impl Add for RoutingCost {
    type Output = RoutingCost;

    fn add(self, rhs: Self) -> Self::Output {
        RoutingCost {
            cost_millis: self.cost_millis + rhs.cost_millis,
            actual_millis: self.actual_millis + rhs.actual_millis,
            distance_mm: self.distance_mm + rhs.distance_mm,
        }
    }
}

impl RoutingCost {
    pub fn zero() -> RoutingCost {
        RoutingCost {
            cost_millis: ElapsedTime::zero(),
            actual_millis: ElapsedTime::zero(),
            distance_mm: TravelledDistance::zero(),
        }
    }

    pub fn elapsed_equivalent(&self) -> ElapsedTime {
        self.cost_millis
    }

    pub fn elapsed_actual(&self) -> ElapsedTime {
        self.actual_millis
    }

    pub fn distance(&self) -> TravelledDistance {
        self.distance_mm
    }

    pub fn with_penalty(&self, penalty_millis: ElapsedTime) -> RoutingCost {
        RoutingCost {
            cost_millis: self.cost_millis + penalty_millis,
            actual_millis: self.actual_millis,
            distance_mm: self.distance_mm,
        }
    }

    pub fn with_additional(
        &self,
        actual_millis: ElapsedTime,
        distance_mm: TravelledDistance,
    ) -> RoutingCost {
        RoutingCost {
            cost_millis: self.cost_millis + actual_millis,
            actual_millis: self.actual_millis + actual_millis,
            distance_mm: self.distance_mm + distance_mm,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TransitionToCost<'a> {
    pub(crate) way_transition: WayTransition,
    pub(crate) from_way_tags: &'a Tags,
    pub(crate) to_way_tags: &'a Tags,
    pub(crate) intersection_tags: &'a Tags,
}

impl<'a> TransitionToCost<'a> {
    pub fn from_way_id(&'a self) -> WayId {
        self.way_transition.from_way_id()
    }

    pub fn to_way_id(&'a self) -> WayId {
        self.way_transition.to_way_id()
    }

    pub fn from_way_tags(&'a self) -> Tags {
        self.from_way_tags.clone()
    }
    pub fn to_way_tags(&'a self) -> Tags {
        self.to_way_tags.clone()
    }
    pub fn intersection_tags(&'a self) -> Tags {
        self.intersection_tags.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TransitionCostResult {
    pub(crate) transition_costs: HashMap<WayId, RoutingCost>,
    pub(crate) continue_cost: Option<RoutingCost>,
}

impl TransitionCostResult {
    pub fn impassable() -> TransitionCostResult {
        TransitionCostResult {
            transition_costs: HashMap::new(),
            continue_cost: None,
        }
    }

    pub fn from_transitions_and_costs_seconds(
        transitions: &HashMap<WayId, f64>,
        continuation_penalty: Option<f64>,
    ) -> TransitionCostResult {
        TransitionCostResult {
            transition_costs: transitions
                .iter()
                .map(|(way_id, penalty_seconds)| {
                    (
                        *way_id,
                        RoutingCost::zero()
                            .with_penalty(ElapsedTime::from_seconds(*penalty_seconds)),
                    )
                })
                .collect(),
            continue_cost: continuation_penalty.map(|penalty_seconds| {
                RoutingCost::zero().with_penalty(ElapsedTime::from_seconds(penalty_seconds))
            }),
        }
    }

    pub(crate) fn zero(transitions: &[WayTransition]) -> TransitionCostResult {
        TransitionCostResult {
            transition_costs: transitions
                .iter()
                .map(|transition| (transition.to_way_id(), RoutingCost::zero()))
                .collect(),
            continue_cost: Some(RoutingCost::zero()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WayCoster {
    speed_forward: Option<TravelSpeed>,
    speed_reverse: Option<TravelSpeed>,
    penalty_ppm_forward: Option<PartsPerMillion>,
    penalty_ppm_reverse: Option<PartsPerMillion>,
}

impl WayCoster {
    pub fn impassable() -> WayCoster {
        WayCoster {
            speed_forward: None,
            speed_reverse: None,
            penalty_ppm_forward: None,
            penalty_ppm_reverse: None,
        }
    }
    pub fn from_speeds(
        speed_forward: Option<TravelSpeed>,
        speed_reverse: Option<TravelSpeed>,
        penalty_ppm_forward: Option<PartsPerMillion>,
        penalty_ppm_reverse: Option<PartsPerMillion>,
    ) -> Self {
        WayCoster {
            speed_forward,
            speed_reverse,
            penalty_ppm_forward,
            penalty_ppm_reverse,
        }
    }

    fn estimate_time_ms(
        &self,
        distance: TravelledDistance,
        direction: Direction,
    ) -> Option<ElapsedTime> {
        distance / self.estimate_speed(direction)?
    }

    fn estimate_speed(&self, direction: Direction) -> Option<TravelSpeed> {
        match direction {
            Direction::Forward => self.speed_forward,
            Direction::Reverse => self.speed_reverse,
        }
    }

    pub fn cost_way_segment(
        &self,
        distance: TravelledDistance,
        direction: Direction,
    ) -> Option<RoutingCost> {
        let penalty_ppm = match direction {
            Direction::Forward => self.penalty_ppm_forward.unwrap_or_default(),
            Direction::Reverse => self.penalty_ppm_reverse.unwrap_or_default(),
        };

        let travel_time_ms = self.estimate_time_ms(distance, direction)?;
        Some(RoutingCost {
            cost_millis: travel_time_ms + travel_time_ms * penalty_ppm,
            actual_millis: travel_time_ms,
            distance_mm: distance,
        })
    }
}

impl ShallowCopy for WayCoster {
    unsafe fn shallow_copy(&self) -> std::mem::ManuallyDrop<Self> {
        ManuallyDrop::new(*self)
    }
}

pub trait CostingModel {
    fn cost_intersection(
        &self,
        current_way_tags: &Tags,
        transitions_to_cost: &[TransitionToCost],
    ) -> TransitionCostResult;

    fn cost_way(&self, tags: &Tags) -> WayCoster;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tags {
    map: HashMap<String, String>,
}

impl Tags {
    pub(super) fn from_hashmap(map: HashMap<String, String>) -> Tags {
        Tags { map }
    }

    pub fn tag_in(&self, key: &str, options: &[&str]) -> bool {
        for actual in self.map.get(key).iter().flat_map(|key| key.split(';')) {
            if options.iter().any(|val| actual == *val) {
                return true;
            }
        }
        false
    }

    pub fn tag_is(&self, key: &str, val: &str) -> bool {
        for actual in self.map.get(key).iter().flat_map(|key| key.split(';')) {
            if actual == val {
                return true;
            }
        }
        false
    }

    pub fn to_hashmap(&self) -> HashMap<String, String> {
        self.map.clone()
    }
}
