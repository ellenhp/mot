use super::{
    CostingModel, RoutingCost,
    base::{BaseCostingModel, WayCost},
    units::{ElapsedTime, TravelSpeed},
};

pub fn pedestrian_costing_model(pedestrian_speed_m_s: f64) -> Box<dyn CostingModel> {
    Box::new(BaseCostingModel::new(
        move |_direction, tags| {
            let mut cost =
                WayCost::from_speed(TravelSpeed::from_meters_per_second(pedestrian_speed_m_s));

            let is_footpath = tags.tag_in("highway", &["footway", "steps"]);
            let has_sidewalk = tags.tag_in("sidewalk", &["both", "left", "right", "yes"]);
            let sidewalk_is_separate = tags.tag_is("sidewalk", "separate")
                || tags.tag_is("sidewalk:left", "separate")
                || tags.tag_is("sidewalk:right", "separate");
            let is_arterial = tags.tag_in("highway", &["secondary", "primary"]);
            let is_highway = tags.tag_in(
                "highway",
                &["motorway", "trunk", "motorway_link", "trunk_link"],
            );

            // Most-preferred.
            if is_footpath {
                return Some(cost);
            }
            // Discourage usage of the main road if there's a separate sidewalk.
            if sidewalk_is_separate {
                cost.add_flat_penalty(ElapsedTime::from_seconds(30));
                cost.add_penalty_ppm(0.2.into());
            }
            if !has_sidewalk {
                if is_highway {
                    // What the hell are you even doing on a highway.
                    cost.add_flat_penalty(ElapsedTime::from_seconds(120));
                    cost.add_penalty_ppm(2.0.into());
                } else {
                    // Slightly discourage use of a road if we don't know for sure that it has a sidewalk.
                    cost.add_flat_penalty(ElapsedTime::from_seconds(10));
                    cost.add_penalty_ppm(0.1.into());
                }
            }

            // Slightly discourage the use of arterials, primarily for noise reasons.
            if is_arterial {
                cost.add_penalty_ppm(0.05.into());
            }
            Some(cost)
        },
        |_tags, _other_intersection_tags| Some(RoutingCost::zero()),
    ))
}
