use std::collections::HashMap;

use super::{
    CostingModel, Direction, RoutingCost, WayCoster,
    units::{PartsPerMillion, TravelSpeed},
};

pub struct WayCost {
    speed: TravelSpeed,
    penalty_ppm: PartsPerMillion,
}

impl WayCost {
    pub fn from_speed(speed: TravelSpeed) -> WayCost {
        WayCost {
            speed,
            penalty_ppm: PartsPerMillion::of(0),
        }
    }
    pub fn with_speed_limit(&self, speed: TravelSpeed) -> WayCost {
        WayCost {
            speed: self.speed.min(speed),
            penalty_ppm: self.penalty_ppm,
        }
    }

    pub fn add_penalty_ppm(&self, penalty: PartsPerMillion) -> WayCost {
        WayCost {
            speed: self.speed,
            penalty_ppm: self.penalty_ppm + penalty,
        }
    }
}

pub struct BaseCostingModel<
    CostWayFn: Fn(Direction, &HashMap<String, String>) -> Option<WayCost>,
    IntersectionFn: Fn(&HashMap<String, String>, &[&HashMap<String, String>]) -> Option<RoutingCost>,
> {
    speed_fn: CostWayFn,
    intersection_fn: IntersectionFn,
}

impl<
    CostWayFn: Fn(Direction, &HashMap<String, String>) -> Option<WayCost>,
    IntersectionFn: Fn(&HashMap<String, String>, &[&HashMap<String, String>]) -> Option<RoutingCost>,
> BaseCostingModel<CostWayFn, IntersectionFn>
{
    pub fn new(
        speed_fn: CostWayFn,
        intersection_fn: IntersectionFn,
    ) -> BaseCostingModel<CostWayFn, IntersectionFn> {
        BaseCostingModel {
            speed_fn,
            intersection_fn,
        }
    }
}

impl<
    CostWayFn: Fn(Direction, &HashMap<String, String>) -> Option<WayCost>,
    IntersectionFn: Fn(&HashMap<String, String>, &[&HashMap<String, String>]) -> Option<RoutingCost>,
> CostingModel for BaseCostingModel<CostWayFn, IntersectionFn>
{
    fn cost_intersection(
        &self,
        tags: &HashMap<String, String>,
        others: &[&HashMap<String, String>],
    ) -> Option<RoutingCost> {
        (self.intersection_fn)(tags, others)
    }

    fn cost_way(&self, tags: &HashMap<String, String>) -> WayCoster {
        let way_cost_forward = (self.speed_fn)(Direction::Forward, tags);
        let way_cost_reverse = (self.speed_fn)(Direction::Reverse, tags);
        WayCoster {
            speed_forward: way_cost_forward.as_ref().map(|wc| wc.speed),
            speed_reverse: way_cost_reverse.as_ref().map(|wc| wc.speed),
            penalty_ppm_forward: way_cost_forward.map(|wc| wc.penalty_ppm),
            penalty_ppm_reverse: way_cost_reverse.map(|wc| wc.penalty_ppm),
        }
    }
}
