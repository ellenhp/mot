use super::{
    CostingModel, RoutingCost,
    base::{BaseCostingModel, WayCost},
    units::TravelSpeed,
};

pub fn pedestrian_costing_model(pedestrian_speed_m_s: f64) -> Box<dyn CostingModel> {
    Box::new(BaseCostingModel::new(
        move |_direction, _tags| {
            Some(WayCost::from_speed(TravelSpeed::from_meters_per_second(
                pedestrian_speed_m_s,
            )))
        },
        |_tags, _other_intersection_tags| Some(RoutingCost::zero()),
    ))
}
