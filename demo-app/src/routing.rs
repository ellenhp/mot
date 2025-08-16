use mvtr::{
    costing::{CostingModel, WayCoster},
    graph::{Graph, WayId},
};
use std::sync::{Mutex, OnceLock};
use wasm_bindgen::prelude::*;
use web_sys::console;

struct JsCostingModel<'a> {
    cost_intersection: &'a js_sys::Function,
    cost_way: &'a js_sys::Function,
}

impl<'a> CostingModel for JsCostingModel<'a> {
    fn cost_intersection(
        &self,
        tags: &mvtr::costing::Tags,
        others: &[&mvtr::costing::Tags],
    ) -> Option<mvtr::costing::RoutingCost> {
        if let Ok(cost) = (self.cost_intersection).call2(
            &JsValue::null(),
            &serde_wasm_bindgen::to_value(tags).unwrap(),
            &serde_wasm_bindgen::to_value(&others.to_vec()).unwrap(),
        ) {
            match serde_wasm_bindgen::from_value(cost) {
                Ok(routing_cost) => return Some(routing_cost),
                Err(_) => return None,
            }
        }
        None
    }

    fn cost_way(&self, tags: &mvtr::costing::Tags) -> mvtr::costing::WayCoster {
        if let Ok(cost) = self.cost_way.call1(
            &JsValue::null(),
            &serde_wasm_bindgen::to_value(tags).unwrap(),
        ) {
            match serde_wasm_bindgen::from_value(cost) {
                Ok(way_coster) => return way_coster,
                Err(_) => return WayCoster::impassable(),
            }
        }
        WayCoster::impassable()
    }
}

static GRAPH: Mutex<OnceLock<Graph>> = Mutex::new(OnceLock::new());

#[wasm_bindgen]
pub fn ingest_tile(
    tile_data: &[u8],
    f: &js_sys::Function,
    g: &js_sys::Function,
) -> Result<(), wasm_bindgen::JsError> {
    console::log_1(&JsValue::from_str("Locking graph"));
    let graph_guard = GRAPH
        .lock()
        .map_err(|_err| JsError::new("Failed to lock mutex"))?;
    let graph = graph_guard.get_or_init(|| Graph::new());
    let costing_model = JsCostingModel {
        cost_intersection: f,
        cost_way: g,
    };

    console::log_1(&JsValue::from_str("Ingesting tile"));
    graph
        .ingest_tile(tile_data.to_vec(), &costing_model)
        .map_err(|err| JsError::new(&format!("Failed to ingest tile: {}", &err)))?;

    Ok(())
}

#[wasm_bindgen]
pub fn search(
    start_way: u64,
    distance_along_start_way: f64,
    end_way: u64,
    distance_along_end_way: f64,
) -> Result<(), wasm_bindgen::JsError> {
    let graph_guard = GRAPH
        .lock()
        .map_err(|_err| JsError::new("Failed to lock mutex"))?;
    let graph = graph_guard.get_or_init(|| Graph::new());
    graph.search_djikstra(
        WayId::from_id(start_way),
        (distance_along_start_way * 1000.0) as i32,
        WayId::from_id(end_way),
        (distance_along_end_way * 1000.0) as i32,
    );
    Ok(())
}
