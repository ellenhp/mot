use mvtr::{
    costing::{
        CostingModel, TransitionCostResult, TransitionToCost, WayCoster,
        units::{PartsPerMillion, TravelSpeed},
    },
    graph::{Graph, WayId},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};
use wasm_bindgen::prelude::*;
use web_sys::console;

struct JsCostingModel<'a> {
    cost_intersection: &'a js_sys::Function,
    cost_way: &'a js_sys::Function,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IntersectionCostInput {
    from_way_id: WayId,
    from_way_tags: HashMap<String, String>,
    to_way_tags: HashMap<String, String>,
    to_way_id: WayId,
    intersection_tags: HashMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IntersectionCostOutputLine {
    to_way_id: u64,
    penalty_seconds: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IntersectionCostOutput {
    pub(crate) transition_costs: Vec<IntersectionCostOutputLine>,
    pub(crate) continue_penalty: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsWayCoster {
    speed_forward_meters_per_second: Option<f64>,
    speed_reverse_meters_per_second: Option<f64>,
    time_penalty_fraction_forward: Option<f64>,
    time_penalty_fraction_reverse: Option<f64>,
}

impl<'a> CostingModel for JsCostingModel<'a> {
    fn cost_intersection(
        &self,
        current_way_tags: &mvtr::costing::Tags,
        intersections_to_cost: &[TransitionToCost],
    ) -> TransitionCostResult {
        let intersections_to_cost: Vec<IntersectionCostInput> = intersections_to_cost
            .iter()
            .map(|transition| IntersectionCostInput {
                from_way_id: transition.from_way_id(),
                from_way_tags: transition.from_way_tags().to_hashmap(),
                to_way_id: transition.to_way_id(),
                to_way_tags: transition.to_way_tags().to_hashmap(),
                intersection_tags: transition.intersection_tags().to_hashmap(),
            })
            .collect();
        match (self.cost_intersection).call2(
            &JsValue::null(),
            &serde_wasm_bindgen::to_value(&current_way_tags.to_hashmap()).unwrap(),
            &serde_wasm_bindgen::to_value(&intersections_to_cost).unwrap(),
        ) {
            Ok(cost) => {
                let output: IntersectionCostOutput = match serde_wasm_bindgen::from_value(cost) {
                    Ok(cost_result) => cost_result,
                    Err(err) => {
                        console::log_1(&JsValue::from_str(&format!(
                            "Intersection cost output didn't match expected schema: {}",
                            err
                        )));
                        return TransitionCostResult::impassable();
                    }
                };

                let transitions_map = output
                    .transition_costs
                    .into_iter()
                    .map(|transition| {
                        (
                            WayId::from_id(transition.to_way_id),
                            transition.penalty_seconds,
                        )
                    })
                    .collect();

                TransitionCostResult::from_transitions_and_costs_seconds(
                    &transitions_map,
                    output.continue_penalty,
                )
            }
            Err(err) => {
                console::log_1(&err);
                TransitionCostResult::impassable()
            }
        }
    }

    fn cost_way(&self, tags: &mvtr::costing::Tags) -> mvtr::costing::WayCoster {
        if let Ok(cost) = self.cost_way.call1(
            &JsValue::null(),
            &serde_wasm_bindgen::to_value(&tags.to_hashmap()).unwrap(),
        ) {
            let js_way_coster: JsWayCoster = match serde_wasm_bindgen::from_value(cost) {
                Ok(way_coster) => way_coster,
                Err(err) => {
                    console::log_1(&JsValue::from_str(&format!(
                        "Way cost output didn't match expected schema: {}",
                        err
                    )));
                    return WayCoster::impassable();
                }
            };

            return WayCoster::from_speeds(
                js_way_coster
                    .speed_forward_meters_per_second
                    .map(|speed| TravelSpeed::from_meters_per_second(speed)),
                js_way_coster
                    .speed_forward_meters_per_second
                    .map(|speed| TravelSpeed::from_meters_per_second(speed)),
                js_way_coster
                    .time_penalty_fraction_forward
                    .map(|fraction| PartsPerMillion::from_fraction(fraction)),
                js_way_coster
                    .time_penalty_fraction_reverse
                    .map(|fraction| PartsPerMillion::from_fraction(fraction)),
            );
        }
        WayCoster::impassable()
    }
}

static GRAPH: Mutex<OnceLock<Graph>> = Mutex::new(OnceLock::new());

#[wasm_bindgen]
pub fn ingest_tile(
    x: u32,
    y: u32,
    z: u32,
    tile_data_ways: &[u8],
    tile_data_nodes: &[u8],
    cost_intersection: &js_sys::Function,
    cost_way: &js_sys::Function,
) -> Result<(), wasm_bindgen::JsError> {
    console::log_1(&JsValue::from_str("Locking graph"));
    let graph_guard = GRAPH
        .lock()
        .map_err(|_err| JsError::new("Failed to lock mutex"))?;
    let graph = graph_guard.get_or_init(|| Graph::new());
    let costing_model = JsCostingModel {
        cost_intersection,
        cost_way,
    };

    console::log_1(&JsValue::from_str("Ingesting tile"));
    graph
        .ingest_tile(
            x,
            y,
            z,
            tile_data_ways.to_vec(),
            tile_data_nodes.to_vec(),
            &costing_model,
        )
        .map_err(|err| JsError::new(&format!("Failed to ingest tile: {}", &err)))?;

    Ok(())
}

#[wasm_bindgen]
pub fn clear() -> Result<(), wasm_bindgen::JsError> {
    let graph_guard = GRAPH
        .lock()
        .map_err(|_err| JsError::new("Failed to lock mutex"))?;
    let graph = graph_guard.get_or_init(|| Graph::new());
    graph
        .clear()
        .map_err(|_err| JsError::new("Failed to clear graph"))?;

    Ok(())
}

#[wasm_bindgen]
pub fn search(
    from_lon: f64,
    from_lat: f64,
    to_lon: f64,
    to_lat: f64,
) -> Result<Option<String>, wasm_bindgen::JsError> {
    let graph_guard = GRAPH
        .lock()
        .map_err(|_err| JsError::new("Failed to lock mutex"))?;
    let graph = graph_guard.get_or_init(|| Graph::new());

    if let (Some((start_way, distance_along_start)), Some((end_way, distance_along_end))) = (
        graph.nearest_way(&geo::Coord {
            x: from_lon,
            y: from_lat,
        }),
        graph.nearest_way(&geo::Coord {
            x: to_lon,
            y: to_lat,
        }),
    ) {
        let response =
            graph.search_djikstra(start_way, distance_along_start, end_way, distance_along_end);
        if response.is_none() {
            console::log_1(&JsValue::from_str("Couldn't find a way there"));
        }
        if let Some(response) = &response {
            console::log_4(
                &JsValue::from_str("Route cost, route duration, route distance: "),
                &JsValue::from_f64(response.route_cost_seconds()),
                &JsValue::from_f64(response.route_duration_seconds()),
                &JsValue::from_f64(response.route_distance_meters()),
            );
        }
        Ok(response.map(|result| result.encoded_polyline()))
    } else {
        Ok(None)
    }
}
