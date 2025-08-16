use super::{
    CostingModel, Direction, Tags, TransitionCostResult, TransitionToCost, WayCoster,
    units::{ElapsedTime, PartsPerMillion, TravelSpeed},
};

pub struct WayCost {
    speed: TravelSpeed,
    penalty_ppm: PartsPerMillion,
    flat_penalty: ElapsedTime,
}

impl WayCost {
    pub fn from_speed(speed: TravelSpeed) -> WayCost {
        WayCost {
            speed,
            penalty_ppm: PartsPerMillion::of(0),
            flat_penalty: ElapsedTime::zero(),
        }
    }
    pub fn limit_speed(&mut self, speed: TravelSpeed) {
        self.speed = self.speed.min(speed);
    }

    pub fn add_penalty_ppm(&mut self, penalty: PartsPerMillion) {
        self.penalty_ppm = self.penalty_ppm + penalty;
    }

    pub fn add_flat_penalty(&mut self, flat_penalty: ElapsedTime) {
        self.flat_penalty = self.flat_penalty + flat_penalty;
    }
}

pub struct BaseCostingModel<
    CostWayFn: Fn(Direction, &Tags) -> Option<WayCost>,
    IntersectionFn: Fn(&Tags, &[TransitionToCost]) -> TransitionCostResult,
> {
    speed_fn: CostWayFn,
    intersection_fn: IntersectionFn,
}

impl<
    CostWayFn: Fn(Direction, &Tags) -> Option<WayCost>,
    IntersectionFn: Fn(&Tags, &[TransitionToCost]) -> TransitionCostResult,
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
    CostWayFn: Fn(Direction, &Tags) -> Option<WayCost>,
    IntersectionFn: Fn(&Tags, &[TransitionToCost]) -> TransitionCostResult,
> CostingModel for BaseCostingModel<CostWayFn, IntersectionFn>
{
    fn cost_intersection(
        &self,
        current_way_tags: &Tags,
        intersecting_way_tags_plus_restrictions: &[TransitionToCost],
    ) -> TransitionCostResult {
        (self.intersection_fn)(current_way_tags, intersecting_way_tags_plus_restrictions)
    }

    fn cost_way(&self, tags: &Tags) -> WayCoster {
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
