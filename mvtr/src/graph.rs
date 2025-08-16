use std::{
    collections::{BTreeMap, BinaryHeap, HashMap, HashSet},
    f64::consts::PI,
    mem::ManuallyDrop,
    sync::Mutex,
};

use evmap::ShallowCopy;
use mvt_reader::feature;
use serde::{Deserialize, Serialize};

use crate::costing::{
    CostingModel, RoutingCost, Tags, TransitionToCost, WayCoster,
    units::{Direction, TravelledDistance},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WayId(u64);

impl WayId {
    pub fn from_id(id: u64) -> WayId {
        WayId(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TileCoordinates {
    x: u32,
    y: u32,
    z: u32,
    extent: u32,
    tile_x: i32,
    tile_y: i32,
}

impl TileCoordinates {
    fn northwest_corner(x: u32, y: u32, z: u32) -> geo::Point {
        let n = 2f64.powi(z as i32);
        let lon_deg = (x as f64) / n * 360.0 - 180.0;
        let lat_rad = f64::atan(f64::sinh(PI * (1.0 - 2.0 * (y as f64) / n)));
        let lat_deg = lat_rad.to_degrees();
        geo::Point::new(lon_deg, lat_deg)
    }

    pub fn tile_envelope(&self) -> geo::Rect {
        let northwest = Self::northwest_corner(self.x, self.y, self.z);
        let southeast = Self::northwest_corner(self.x + 1, self.y + 1, self.z);
        geo::Rect::new(northwest.0, southeast.0)
    }

    pub fn to_lat_lng(&self) -> geo::Point {
        let envelope = self.tile_envelope();
        let x_frac = self.tile_x as f64 / self.extent as f64;
        let y_frac = self.tile_y as f64 / self.extent as f64;
        let x = envelope.min().x + x_frac * envelope.width();
        let y = envelope.min().y + y_frac * envelope.height();
        geo::Point::new(x, y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub(crate) struct CostedWayTransition {
    way_transition: WayTransition,
    cost: RoutingCost,
}

impl evmap::ShallowCopy for CostedWayTransition {
    unsafe fn shallow_copy(&self) -> std::mem::ManuallyDrop<Self> {
        ManuallyDrop::new(*self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct WayTransition {
    distance_along_way_mm: i32,
    to_way_id: WayId,
    transition_to_distance_along_way_mm: i32,
}

impl PartialOrd for WayTransition {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self
            .distance_along_way_mm
            .partial_cmp(&other.distance_along_way_mm)
        {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.to_way_id.partial_cmp(&other.to_way_id) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.transition_to_distance_along_way_mm
            .partial_cmp(&other.transition_to_distance_along_way_mm)
    }
}

impl Ord for WayTransition {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

fn meters_to_mm_fixed(meters: f32) -> i32 {
    (meters * 1000.0) as i32
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct SearchNode {
    way: WayId,
    distance_along_way_mm: i32,
}

impl ShallowCopy for SearchNode {
    unsafe fn shallow_copy(&self) -> ManuallyDrop<Self> {
        ManuallyDrop::new(*self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SearchState {
    previous: usize,
    idx: usize,
    node: SearchNode,
    cost: RoutingCost,
}

impl PartialOrd for SearchState {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.cost.partial_cmp(&other.cost) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord.map(|ord| ord.reverse()),
        }
        self.node.partial_cmp(&other.node)
    }
}

impl Ord for SearchState {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

pub struct Graph {
    nodes_read: evmap::ReadHandle<WayId, SearchNode>,
    nodes_write: Mutex<evmap::WriteHandle<WayId, SearchNode>>,
    transitions_read: evmap::ReadHandle<SearchNode, CostedWayTransition>,
    transitions_write: Mutex<evmap::WriteHandle<SearchNode, CostedWayTransition>>,
    ways_read: evmap::ReadHandle<WayId, WayCoster>,
    ways_write: Mutex<evmap::WriteHandle<WayId, WayCoster>>,
}

impl Graph {
    pub fn new() -> Graph {
        let (nr, nw) = evmap::new();
        let (tr, tw) = evmap::new();
        let (wr, ww) = evmap::new();
        Graph {
            nodes_read: nr,
            nodes_write: Mutex::new(nw),
            transitions_read: tr,
            transitions_write: Mutex::new(tw),
            ways_read: wr,
            ways_write: Mutex::new(ww),
        }
    }

    pub fn ingest_tile<CM: CostingModel>(
        &self,
        _x: u32,
        _y: u32,
        _z: u32,
        mvt: Vec<u8>,
        costing_model: &CM,
    ) -> anyhow::Result<()> {
        struct AnnotatedWayTransition<'a> {
            way_transition: WayTransition,
            way_tags: &'a Tags,
            other_way_tags: &'a Tags,
            intersection_tags: Tags,
        }

        let reader = mvt_reader::Reader::new(mvt)
            .map_err(|err| anyhow::anyhow!("Could not create MVT reader {}", err))?;
        let layers = reader
            .get_layer_names()
            .map_err(|err| anyhow::anyhow!("Could not get MVT tile's layer list {}", err))?;

        let mut way_tags: HashMap<WayId, Tags> = HashMap::new();
        if let Some((road_layer_id, _)) = layers
            .iter()
            .enumerate()
            .filter(|(_, layer)| layer.as_str() == "roads")
            .next()
        {
            let features = reader
                .get_features(road_layer_id)
                .map_err(|err| anyhow::anyhow!("Could not get MVT tile's road features {}", err))?;

            for feature in &features {
                let _props_default = HashMap::new();
                let properties = feature.properties.as_ref().unwrap_or(&_props_default);

                let way_id = WayId(Self::get_u64_property(properties, "way_id")?);
                let mut tags = HashMap::new();
                for (key, value) in feature.properties.as_ref().unwrap_or(&HashMap::new()) {
                    match value {
                        mvt_reader::feature::Value::String(value) => {
                            tags.insert(key.clone(), value.clone())
                        }
                        _ => continue,
                    };
                }
                let tags = Tags::from_hashmap(tags);
                let way_cost = costing_model.cost_way(&tags);
                way_tags.insert(way_id, tags);
                self.ways_write
                    .lock()
                    .map_err(|err| anyhow::anyhow!("Failed to lock mutex: {}", err))?
                    .insert(way_id, way_cost);
            }
        }
        if let Some((intersection_layer_id, _)) = layers
            .iter()
            .enumerate()
            .filter(|(_, layer)| layer.as_str() == "intersections")
            .next()
        {
            let features = reader.get_features(intersection_layer_id).map_err(|err| {
                anyhow::anyhow!("Could not get MVT tile's intersection features {}", err)
            })?;

            let mut transition_groups: HashMap<SearchNode, Vec<AnnotatedWayTransition>> =
                HashMap::new();
            for feature in &features {
                let _props_default = HashMap::new();
                let properties = feature.properties.as_ref().unwrap_or(&_props_default);

                let mut intersection_tags = HashMap::new();
                for (key, value) in feature.properties.as_ref().unwrap_or(&HashMap::new()) {
                    match value {
                        mvt_reader::feature::Value::String(value) => {
                            intersection_tags.insert(key.clone(), value.clone())
                        }
                        _ => continue,
                    };
                }
                let intersection_tags = Tags::from_hashmap(intersection_tags);

                let from_way_id = WayId(Self::get_u64_property(properties, "way_id")?);
                let to_way_id = WayId(Self::get_u64_property(properties, "transition_to_way")?);
                let distance_along_way = Self::get_f32_property(properties, "distance_along_way")?;
                let transition_to_distance_along_way =
                    Self::get_f32_property(properties, "transition_to_distance_along_way")?;

                let distance_along_way_mm = meters_to_mm_fixed(distance_along_way);
                let search_node = SearchNode {
                    way: from_way_id,
                    distance_along_way_mm,
                };
                let way_transition = WayTransition {
                    distance_along_way_mm,
                    transition_to_distance_along_way_mm: meters_to_mm_fixed(
                        transition_to_distance_along_way,
                    ),
                    to_way_id: to_way_id,
                };
                self.nodes_write
                    .lock()
                    .map_err(|err| anyhow::anyhow!("Failed to lock mutex: {}", err))?
                    .insert(from_way_id, search_node);

                // let tags =
                let annotated_way_transition = AnnotatedWayTransition {
                    way_transition,
                    way_tags: way_tags
                        .get(&from_way_id)
                        .expect("Missing way tags for from_way"),
                    other_way_tags: way_tags
                        .get(&to_way_id)
                        .expect("Missing way tags for from_way"),
                    intersection_tags,
                };

                if let Some(transitions) = transition_groups.get_mut(&search_node) {
                    transitions.push(annotated_way_transition);
                } else {
                    transition_groups.insert(search_node, vec![annotated_way_transition]);
                }
            }

            for (search_node, transition_group) in transition_groups {
                let current_way_tags = way_tags.get(&search_node.way).expect("Missing way tags");
                let intersecting_way_tags_plus_restrictions: Vec<TransitionToCost> =
                    transition_group
                        .iter()
                        .map(|transition| TransitionToCost {
                            way_transition: transition.way_transition,
                            from_way_tags: transition.way_tags,
                            to_way_tags: transition.other_way_tags,
                            intersection_tags: &transition.intersection_tags,
                        })
                        .collect();

                let intersection_costs = costing_model
                    .cost_intersection(current_way_tags, &intersecting_way_tags_plus_restrictions);

                for (way_transition, transition_cost) in &intersection_costs.transition_costs {
                    let costed_way_transition = CostedWayTransition {
                        way_transition: *way_transition,
                        cost: *transition_cost,
                    };
                    self.transitions_write
                        .lock()
                        .map_err(|err| anyhow::anyhow!("Failed to lock mutex: {}", err))?
                        .insert(search_node, costed_way_transition);
                }
                // Insert an identity transition to represent the cost interacting with the intersection and continuing along the same way.
                if let Some(continue_cost) = intersection_costs.continue_cost {
                    let costed_way_transition = CostedWayTransition {
                        way_transition: WayTransition {
                            distance_along_way_mm: search_node.distance_along_way_mm,
                            to_way_id: search_node.way,
                            transition_to_distance_along_way_mm: search_node.distance_along_way_mm,
                        },
                        cost: continue_cost,
                    };
                    self.transitions_write
                        .lock()
                        .map_err(|err| anyhow::anyhow!("Failed to lock mutex: {}", err))?
                        .insert(search_node, costed_way_transition);
                }
            }

            // for (search_node, transition_group) in transition_groups {
            //     let others_tags: Vec<Tags> = transition_group
            //         .iter()
            //         .enumerate()
            //         .filter(|(other_idx, other_transition)| *other_idx != idx)
            //         .map(|(_, transition)| transition.tags.clone())
            //         .collect();

            //     for (idx, annotated_transition) in transition_group.iter().enumerate() {
            //         let others_tags: Vec<Tags> = transition_group
            //             .iter()
            //             .enumerate()
            //             .filter(|(other_idx, other_transition)| *other_idx != idx)
            //             .map(|(_, transition)| transition.tags.clone())
            //             .collect();
            //         let mut transition = annotated_transition.way_transition.clone();
            //         transition.transition_cost = costing_model.cost_intersection(tags, others);
            //         self.transitions_write
            //             .lock()
            //             .map_err(|err| anyhow::anyhow!("Failed to lock mutex: {}", err))?
            //             .insert(search_node, transition);
            //     }
            // }
        }
        // We want costing data to be available before the routing graph is because that way we can unwrap() costing access.
        self.ways_write
            .lock()
            .map_err(|err| anyhow::anyhow!("Failed to lock mutex: {}", err))?
            .refresh();
        self.transitions_write
            .lock()
            .map_err(|err| anyhow::anyhow!("Failed to lock mutex: {}", err))?
            .refresh();
        self.nodes_write
            .lock()
            .map_err(|err| anyhow::anyhow!("Failed to lock mutex: {}", err))?
            .refresh();
        Ok(())
    }

    pub fn search_djikstra(
        &self,
        start: WayId,
        distance_along_start_mm: i32,
        end: WayId,
        distance_along_end_mm: i32,
    ) -> Option<Vec<SearchState>> {
        let first_state = SearchState {
            previous: 0,
            idx: 0,
            node: SearchNode {
                way: start,
                distance_along_way_mm: distance_along_start_mm,
            },
            cost: RoutingCost::zero(),
        };
        let mut frontier = BinaryHeap::new();
        frontier.push(first_state);
        let mut costs: HashMap<SearchNode, RoutingCost> = HashMap::new();
        let mut step_log: Vec<SearchState> = vec![first_state];

        while let Some(state) = frontier.pop() {
            if state.node.way == end && state.node.distance_along_way_mm == distance_along_end_mm {
                return Some(self.unwind_route(&step_log, state.idx));
            }
            let all_nodes: Vec<SearchNode> = self
                .nodes_read
                .get(&state.node.way)
                .iter()
                .flatten()
                .cloned()
                .collect();

            let mut transition_groups = BTreeMap::new();
            for node in &all_nodes {
                let transitions: Vec<CostedWayTransition> = self
                    .transitions_read
                    .get(node)
                    .iter()
                    .flatten()
                    .cloned()
                    .collect();
                transition_groups.insert(node, transitions);
            }
            let first_transition_group_after: Option<Vec<CostedWayTransition>> = transition_groups
                .iter()
                .filter(|(node, _)| node > &&&state.node)
                .map(|(_, v)| v.clone())
                .next();
            let first_transition_group_before: Option<Vec<CostedWayTransition>> = transition_groups
                .iter()
                .rev()
                .filter(|(node, _)| node < &&&state.node)
                .map(|(_, v)| v.clone())
                .next();

            if let Some(group) = first_transition_group_after {
                self.process_transition_set(
                    &group,
                    &state,
                    &mut frontier,
                    &mut costs,
                    &mut step_log,
                );
            }
            if let Some(group) = first_transition_group_before {
                self.process_transition_set(
                    &group,
                    &state,
                    &mut frontier,
                    &mut costs,
                    &mut step_log,
                );
            }
        }
        None
    }

    fn process_transition_set(
        &self,
        transitions: &[CostedWayTransition],
        state: &SearchState,
        frontier: &mut BinaryHeap<SearchState>,
        costs: &mut HashMap<SearchNode, RoutingCost>,
        step_log: &mut Vec<SearchState>,
    ) {
        let first_transition = if let Some(first_transition) = transitions.first() {
            first_transition
        } else {
            tracing::warn!("Empty transition set.");
            return;
        };
        debug_assert!(
            transitions
                .iter()
                .all(|transition| transition.way_transition.distance_along_way_mm
                    == first_transition.way_transition.distance_along_way_mm)
        );

        // let next_nodes: Vec<SearchNode> = transitions
        //     .iter()
        //     .map(|transition| SearchNode {
        //         way: WayId(transition.to_way_id),
        //         distance_along_way_mm: transition.transition_to_distance_along_way_mm,
        //     })
        //     .collect();

        let distance = TravelledDistance(
            (state.node.distance_along_way_mm
                - first_transition.way_transition.distance_along_way_mm)
                .saturating_abs()
                .try_into()
                .expect("Distance was negative after an `abs` call."),
        );
        let direction = if state.node.distance_along_way_mm
            < first_transition.way_transition.distance_along_way_mm
        {
            Direction::Forward
        } else {
            Direction::Reverse
        };

        let segment_cost = if let Some(segment_cost) = self
            .ways_read
            .get_one(&state.node.way)
            .expect("Costing for way not available.")
            .cost_way_segment(distance, direction)
        {
            segment_cost
        } else {
            // Impassable way segment.
            return;
        };

        // Apply the travel cost.
        let new_cost = state.cost + segment_cost;

        for transition in transitions {
            let new_node = SearchNode {
                way: transition.way_transition.to_way_id,
                distance_along_way_mm: transition
                    .way_transition
                    .transition_to_distance_along_way_mm,
            };

            // Apply the transition cost.
            let new_state = SearchState {
                previous: state.idx,
                idx: step_log.len(),
                node: new_node,
                cost: new_cost + transition.cost,
            };

            if let Some(best_cost_this_node) = costs.get_mut(&new_node) {
                if new_state.cost < *best_cost_this_node {
                    frontier.push(new_state);
                    *best_cost_this_node = new_state.cost;
                    step_log.push(new_state);
                }
            } else {
                frontier.push(new_state);
                costs.insert(new_node, new_state.cost);
                step_log.push(new_state);
            }
        }
    }

    fn unwind_route(&self, step_log: &[SearchState], end_step: usize) -> Vec<SearchState> {
        let mut cycle_detector = HashSet::new();
        let mut steps_reversed = Vec::new();
        let mut cursor = end_step;
        while let Some(step) = step_log.get(cursor) {
            if cycle_detector.contains(step) {
                tracing::error!("Cycle detected while unwinding the route");
                return Vec::new();
            }
            cycle_detector.insert(step);
            steps_reversed.push(*step);

            if step.previous == step.idx {
                break;
            }
            cursor = step.previous
        }
        steps_reversed.into_iter().rev().collect()
    }

    fn get_u64_property(
        properties: &HashMap<String, feature::Value>,
        prop_name: &str,
    ) -> anyhow::Result<u64> {
        let prop = properties
            .get(prop_name)
            .ok_or_else(|| anyhow::anyhow!("Intersection missing {prop_name}"))?;
        if let feature::Value::UInt(prop) = prop {
            Ok(*prop)
        } else {
            anyhow::bail!("Property not a u64")
        }
    }

    fn get_f32_property(
        properties: &HashMap<String, feature::Value>,
        prop_name: &str,
    ) -> anyhow::Result<f32> {
        let prop = properties
            .get(prop_name)
            .ok_or_else(|| anyhow::anyhow!("Intersection missing {prop_name}"))?;
        if let feature::Value::Float(prop) = prop {
            Ok(*prop)
        } else {
            anyhow::bail!("Property not a f32")
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Instant;

    use crate::costing::pedestrian::pedestrian_costing_model;

    use super::Graph;

    #[test]
    fn ingest_tile() {
        tracing_subscriber::fmt().init();
        let costing_model = pedestrian_costing_model(1.4);
        let graph = Graph::new();
        let start = Instant::now();
        graph
            .ingest_tile(
                2625,
                5721,
                14,
                include_bytes!("../testdata/tile.pbf").to_vec(),
                &costing_model,
            )
            .expect("Failed to ingest tile");
        dbg!(start.elapsed());
    }

    #[test]
    fn search_basic() {
        tracing_subscriber::fmt().init();
        let costing_model = pedestrian_costing_model(1.4);
        let graph = Graph::new();
        graph
            .ingest_tile(
                2625,
                5721,
                14,
                include_bytes!("../testdata/tile.pbf").to_vec(),
                &costing_model,
            )
            .expect("Failed to ingest tile");
        // approx: https://maps.earth/directions/walk/-122.315503,47.6163794/-122.3126740,47.6153470
        // ----> 325.32080857991474 meters
        let route = graph
            .search_djikstra(super::WayId(1173831634), 0, super::WayId(1172841584), 0)
            .expect("Couldn't find a route.");
        dbg!(&route);
        assert_eq!(route.last().unwrap().cost.distance().mm(), 325_931);
    }

    #[test]
    fn search_fremont() {
        tracing_subscriber::fmt().init();
        let costing_model = pedestrian_costing_model(1.4);
        let graph = Graph::new();
        graph
            .ingest_tile(
                2623,
                5718,
                14,
                include_bytes!("../testdata/tile2.pbf").to_vec(),
                &costing_model,
            )
            .expect("Failed to ingest tile");
        let route = graph
            .search_djikstra(super::WayId(671949014), 0, super::WayId(980366562), 0)
            .expect("Couldn't find a route.");
        dbg!(&route);
        assert_eq!(route.last().unwrap().cost.distance().mm(), 1_996_587);
    }
}
