pub mod base;
pub mod pedestrian;
pub mod units;

use std::{collections::HashMap, mem::ManuallyDrop, ops::Add};

use evmap::ShallowCopy;
use units::{Direction, ElapsedTime, PartsPerMillion, TravelSpeed, TravelledDistance};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WayCoster {
    speed_forward: Option<TravelSpeed>,
    speed_reverse: Option<TravelSpeed>,
    penalty_ppm_forward: Option<PartsPerMillion>,
    penalty_ppm_reverse: Option<PartsPerMillion>,
}

impl WayCoster {
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
        from_distance_along_way_mm: i32,
        to_distance_along_way_mm: i32,
    ) -> Option<RoutingCost> {
        let distance = TravelledDistance(
            (from_distance_along_way_mm - to_distance_along_way_mm)
                .saturating_abs()
                .try_into()
                .expect("Distance was negative after an `abs` call."),
        );
        let direction = if to_distance_along_way_mm < from_distance_along_way_mm {
            Direction::Reverse
        } else {
            Direction::Forward
        };

        let penalty_ppm = match direction {
            Direction::Forward => self.penalty_ppm_forward.unwrap_or_default(),
            Direction::Reverse => self.penalty_ppm_reverse.unwrap_or_default(),
        };

        let time_ms = self.estimate_time_ms(distance, direction)?;
        Some(RoutingCost {
            cost_millis: time_ms + time_ms * penalty_ppm,
            actual_millis: time_ms,
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
    fn cost_intersection(&self, tags: &Tags, others: &[&Tags]) -> Option<RoutingCost>;

    fn cost_way(&self, tags: &Tags) -> WayCoster;
}

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
}
