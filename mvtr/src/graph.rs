use std::{
    collections::{BTreeMap, BinaryHeap, HashMap, HashSet},
    f64::consts::PI,
    mem::ManuallyDrop,
    sync::Mutex,
};

use evmap::ShallowCopy;
use geo::{Haversine, InterpolateLine, LineLocatePoint, Point};
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

    pub fn to_lat_lng(&self) -> geo::Coord {
        let envelope = self.tile_envelope();
        let x_frac = self.tile_x as f64 / self.extent as f64;
        let y_frac = 1.0 - self.tile_y as f64 / self.extent as f64;
        let x = envelope.min().x + x_frac * envelope.width();
        let y = envelope.min().y + y_frac * envelope.height();
        geo::Coord { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub(crate) struct CostedWayTransition {
    to_way_id: WayId,
    cost: RoutingCost,
}

impl evmap::ShallowCopy for CostedWayTransition {
    unsafe fn shallow_copy(&self) -> std::mem::ManuallyDrop<Self> {
        ManuallyDrop::new(*self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct WayTransition {
    from_way_id: WayId,
    distance_along_way_mm: i32,
    to_way_id: WayId,
    transition_to_distance_along_way_mm: i32,
}

impl WayTransition {
    pub fn from_way_id(&self) -> WayId {
        self.from_way_id
    }
    pub fn distance_along_way_mm(&self) -> i32 {
        self.distance_along_way_mm
    }
    pub fn to_way_id(&self) -> WayId {
        self.to_way_id
    }
    pub fn transition_to_distance_along_way_mm(&self) -> i32 {
        self.transition_to_distance_along_way_mm
    }
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

impl ShallowCopy for WayTransition {
    unsafe fn shallow_copy(&self) -> ManuallyDrop<Self> {
        ManuallyDrop::new(*self)
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
struct SearchState {
    previous: usize,
    idx: usize,
    node: SearchNode,
    via: SearchNode,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SearchResult {
    encoded_polyline: String,
    cost: RoutingCost,
}

impl SearchResult {
    pub fn encoded_polyline(&self) -> String {
        self.encoded_polyline.clone()
    }

    pub fn route_distance_meters(&self) -> f64 {
        self.cost.distance().mm() as f64 / 1000.0
    }

    pub fn route_cost_seconds(&self) -> f64 {
        self.cost.elapsed_equivalent().millis() as f64 / 1000.0
    }

    pub fn route_duration_seconds(&self) -> f64 {
        self.cost.elapsed_actual().millis() as f64 / 1000.0
    }
}

pub struct Graph {
    nodes_read: evmap::ReadHandle<WayId, SearchNode>,
    nodes_write: Mutex<evmap::WriteHandle<WayId, SearchNode>>,
    transitions_read: evmap::ReadHandle<SearchNode, (CostedWayTransition, WayTransition)>,
    transitions_write: Mutex<evmap::WriteHandle<SearchNode, (CostedWayTransition, WayTransition)>>,
    ways_read: evmap::ReadHandle<WayId, WayCoster>,
    ways_write: Mutex<evmap::WriteHandle<WayId, WayCoster>>,
    geometry_read: evmap::ReadHandle<WayId, Vec<TileCoordinates>>,
    geometry_write: Mutex<evmap::WriteHandle<WayId, Vec<TileCoordinates>>>,
}

impl Graph {
    pub fn new() -> Graph {
        let (nr, nw) = evmap::new();
        let (tr, tw) = evmap::new();
        let (wr, ww) = evmap::new();
        let (gr, gw) = evmap::new();
        Graph {
            nodes_read: nr,
            nodes_write: Mutex::new(nw),
            transitions_read: tr,
            transitions_write: Mutex::new(tw),
            ways_read: wr,
            ways_write: Mutex::new(ww),
            geometry_read: gr,
            geometry_write: Mutex::new(gw),
        }
    }

    pub fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.ways_write
            .lock()
            .map_err(|err| anyhow::anyhow!("Failed to lock mutex: {}", err))?
            .purge();
        self.geometry_write
            .lock()
            .map_err(|err| anyhow::anyhow!("Failed to lock mutex: {}", err))?
            .purge();
        self.transitions_write
            .lock()
            .map_err(|err| anyhow::anyhow!("Failed to lock mutex: {}", err))?
            .purge();
        self.nodes_write
            .lock()
            .map_err(|err| anyhow::anyhow!("Failed to lock mutex: {}", err))?
            .purge();
        self.ways_write
            .lock()
            .map_err(|err| anyhow::anyhow!("Failed to lock mutex: {}", err))?
            .refresh();
        self.geometry_write
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

    pub fn ingest_tile<CM: CostingModel>(
        &self,
        x: u32,
        y: u32,
        z: u32,
        mvt_ways: Vec<u8>,
        mvt_nodes: Vec<u8>,
        costing_model: &CM,
    ) -> anyhow::Result<()> {
        struct AnnotatedWayTransition<'a> {
            way_transition: WayTransition,
            way_tags: &'a Tags,
            other_way_tags: &'a Tags,
            intersection_tags: Tags,
        }

        let reader_ways = mvt_reader::Reader::new(mvt_ways)
            .map_err(|err| anyhow::anyhow!("Could not create MVT reader {}", err))?;
        let layers_ways = reader_ways
            .get_layer_names()
            .map_err(|err| anyhow::anyhow!("Could not get MVT tile's layer list {}", err))?;

        let mut way_tags: HashMap<WayId, Tags> = HashMap::new();
        if let Some((road_layer_id, _)) = layers_ways
            .iter()
            .enumerate()
            .filter(|(_, layer)| layer.as_str() == "roads")
            .next()
        {
            let features = reader_ways
                .get_features(road_layer_id)
                .map_err(|err| anyhow::anyhow!("Could not get MVT tile's road features {}", err))?;

            let extent = reader_ways
                .get_layer_metadata()
                .map_err(|err| anyhow::anyhow!("Could not get MVT tile's road metadata {}", err))?
                [road_layer_id]
                .extent;

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

                let geometry = match &feature.geometry {
                    geo::Geometry::LineString(line_string) => line_string.clone(),
                    geo::Geometry::MultiLineString(multi_line_string) => {
                        if multi_line_string.0.len() > 1 {
                            tracing::warn!("Multiple linestrings found");
                            continue;
                        }
                        if let Some(linestring) = multi_line_string.0.first() {
                            linestring.clone()
                        } else {
                            tracing::warn!("Zero linestrings found");
                            continue;
                        }
                    }
                    _ => {
                        tracing::warn!("Way geometry was not linestring or multilinestring");
                        continue;
                    }
                };
                let mut polyline = Vec::new();
                for coord in &geometry.0 {
                    polyline.push(TileCoordinates {
                        x,
                        y,
                        z,
                        extent,
                        tile_x: coord.x as i32,
                        tile_y: coord.y as i32,
                    });
                }
                self.geometry_write
                    .lock()
                    .map_err(|err| anyhow::anyhow!("Failed to lock mutex: {}", err))?
                    .insert(way_id, polyline);
            }
        }
        let reader_nodes = mvt_reader::Reader::new(mvt_nodes)
            .map_err(|err| anyhow::anyhow!("Could not create MVT reader {}", err))?;
        let layers_nodes = reader_nodes
            .get_layer_names()
            .map_err(|err| anyhow::anyhow!("Could not get MVT tile's layer list {}", err))?;

        if let Some((intersection_layer_id, _)) = layers_nodes
            .iter()
            .enumerate()
            .filter(|(_, layer)| layer.as_str() == "intersections")
            .next()
        {
            let features = reader_nodes
                .get_features(intersection_layer_id)
                .map_err(|err| {
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
                    from_way_id,
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
                let annotated_way_transition = if let (Some(way_tags), Some(other_way_tags)) =
                    (way_tags.get(&from_way_id), way_tags.get(&to_way_id))
                {
                    AnnotatedWayTransition {
                        way_transition,
                        way_tags: way_tags,
                        other_way_tags: other_way_tags,
                        intersection_tags,
                    }
                } else {
                    tracing::warn!(
                        "Missing way tags for one or more ways, while attempting to cost intersection"
                    );
                    continue;
                };

                if let Some(transitions) = transition_groups.get_mut(&search_node) {
                    transitions.push(annotated_way_transition);
                } else {
                    transition_groups.insert(search_node, vec![annotated_way_transition]);
                }
            }

            for (search_node, transition_group) in transition_groups {
                let way_transition_lookup: HashMap<WayId, WayTransition> = transition_group
                    .iter()
                    .map(|transition| {
                        (
                            transition.way_transition.to_way_id.clone(),
                            transition.way_transition,
                        )
                    })
                    .collect();
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

                for (to_way_id, transition_cost) in &intersection_costs.transition_costs {
                    let costed_way_transition = CostedWayTransition {
                        to_way_id: *to_way_id,
                        cost: *transition_cost,
                    };
                    self.transitions_write
                        .lock()
                        .map_err(|err| anyhow::anyhow!("Failed to lock mutex: {}", err))?
                        .insert(
                            search_node,
                            (
                                costed_way_transition,
                                *way_transition_lookup.get(to_way_id).unwrap(),
                            ),
                        );
                }
                // Insert an identity transition to represent the cost interacting with the intersection and continuing along the same way.
                if let Some(continue_cost) = intersection_costs.continue_cost {
                    let costed_way_transition = CostedWayTransition {
                        to_way_id: search_node.way,
                        cost: continue_cost,
                    };
                    self.transitions_write
                        .lock()
                        .map_err(|err| anyhow::anyhow!("Failed to lock mutex: {}", err))?
                        .insert(
                            search_node,
                            (
                                costed_way_transition,
                                WayTransition {
                                    from_way_id: search_node.way,
                                    distance_along_way_mm: search_node.distance_along_way_mm,
                                    to_way_id: search_node.way,
                                    transition_to_distance_along_way_mm: search_node
                                        .distance_along_way_mm,
                                },
                            ),
                        );
                }
            }
        }
        // We want costing data to be available before the routing graph is because that way we can unwrap() costing access.
        self.ways_write
            .lock()
            .map_err(|err| anyhow::anyhow!("Failed to lock mutex: {}", err))?
            .refresh();
        self.geometry_write
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

    pub fn get_polyline(&self, way: &WayId) -> Option<geo::LineString> {
        let geometry_guard = self.geometry_read.get_one(way)?;
        Some(
            geometry_guard
                .iter()
                .map(|coords| coords.to_lat_lng())
                .collect(),
        )
    }

    pub fn search_djikstra(
        &self,
        start: WayId,
        distance_along_start_mm: i32,
        end: WayId,
        distance_along_end_mm: i32,
    ) -> Option<SearchResult> {
        let states =
            self.search_djikstra_inner(start, distance_along_start_mm, end, distance_along_end_mm)?;
        let cost = states.last()?.cost;

        let mut route_polyline = Vec::new();

        for window in states.windows(2) {
            let state = window[0];

            let node_linestring = self.get_polyline(&state.node.way)?;

            let start_point = Haversine
                .point_at_distance_from_start(
                    &node_linestring,
                    window[0].node.distance_along_way_mm as f64 / 1000.0,
                )
                .expect("Failed to interpolate along way polyline.");
            let end_point = Haversine
                .point_at_distance_from_start(
                    &node_linestring,
                    window[1].via.distance_along_way_mm as f64 / 1000.0,
                )
                .expect("Failed to interpolate along way polyline.");

            let line_fraction_1 = node_linestring.line_locate_point(&start_point).unwrap();
            let line_fraction_2 = node_linestring.line_locate_point(&end_point).unwrap();
            let start_line_fraction = line_fraction_1.min(line_fraction_2);
            let end_line_fraction = line_fraction_1.max(line_fraction_2);

            if route_polyline.last() != Some(&start_point) {
                route_polyline.push(start_point);
            }

            let mut middle_points: Vec<Point> = node_linestring
                .coords()
                .map(|coord| Point(*coord))
                .skip_while(|point| {
                    node_linestring.line_locate_point(point).unwrap() < start_line_fraction
                })
                .take_while(|point| {
                    node_linestring.line_locate_point(point).unwrap() < end_line_fraction
                })
                .collect();
            if line_fraction_1 > line_fraction_2 {
                middle_points.reverse();
            }
            route_polyline.extend_from_slice(&middle_points);

            if route_polyline.last() != Some(&end_point) {
                route_polyline.push(end_point);
            }
        }

        Some(SearchResult {
            cost,
            encoded_polyline: polyline::encode_coordinates(
                route_polyline.iter().map(|point| point.0),
                5,
            )
            .unwrap(),
        })
    }

    fn search_djikstra_inner(
        &self,
        start: WayId,
        distance_along_start_mm: i32,
        end: WayId,
        distance_along_end_mm: i32,
    ) -> Option<Vec<SearchState>> {
        let first_node = SearchNode {
            way: start,
            distance_along_way_mm: distance_along_start_mm,
        };
        let first_state = SearchState {
            previous: 0,
            idx: 0,
            node: first_node,
            via: first_node,
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
            let all_nodes: HashSet<SearchNode> = self
                .nodes_read
                .get(&state.node.way)
                .iter()
                .flatten()
                .cloned()
                .collect();

            debug_assert!(all_nodes.iter().all(|node| node.way == state.node.way));

            let mut transition_groups = BTreeMap::new();
            for node in &all_nodes {
                let transitions: Vec<(CostedWayTransition, WayTransition)> = self
                    .transitions_read
                    .get(node)
                    .iter()
                    .flatten()
                    .cloned()
                    .collect();
                debug_assert!(!transition_groups.contains_key(&node));
                transition_groups.insert(node, transitions);
            }

            debug_assert!(
                transition_groups
                    .keys()
                    .all(|key| key.way == state.node.way)
            );

            let identity_transitions_group = transition_groups
                .iter()
                .filter(|(node, _)| node == &&&state.node)
                .map(|(k, v)| (**k, v.clone()))
                .next();
            let first_transition_group_after = transition_groups
                .iter()
                .filter(|(node, _)| node > &&&state.node)
                .map(|(k, v)| (**k, v.clone()))
                .next();
            let first_transition_group_before = transition_groups
                .iter()
                .filter(|(node, _)| node < &&&state.node)
                .map(|(k, v)| (**k, v.clone()))
                .last();

            if let Some((via, group)) = identity_transitions_group {
                self.process_transition_set(
                    &group,
                    &via,
                    &state,
                    &mut frontier,
                    &mut costs,
                    &mut step_log,
                );
            }
            if let Some((via, group)) = first_transition_group_after {
                self.process_transition_set(
                    &group,
                    &via,
                    &state,
                    &mut frontier,
                    &mut costs,
                    &mut step_log,
                );
            }
            if let Some((via, group)) = first_transition_group_before {
                self.process_transition_set(
                    &group,
                    &via,
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
        costed_transitions: &[(CostedWayTransition, WayTransition)],
        via: &SearchNode,
        state: &SearchState,
        frontier: &mut BinaryHeap<SearchState>,
        costs: &mut HashMap<SearchNode, RoutingCost>,
        step_log: &mut Vec<SearchState>,
    ) {
        let distance: TravelledDistance = TravelledDistance(
            (state.node.distance_along_way_mm - via.distance_along_way_mm)
                .saturating_abs()
                .try_into()
                .expect("Distance was negative after an `abs` call."),
        );
        debug_assert_eq!(state.node.way, via.way);
        let direction = if state.node.distance_along_way_mm < via.distance_along_way_mm {
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

        for (costed, transition) in costed_transitions {
            let new_node = SearchNode {
                way: transition.to_way_id,
                distance_along_way_mm: transition.transition_to_distance_along_way_mm,
            };

            // Apply the transition cost.
            let new_state = SearchState {
                previous: state.idx,
                idx: step_log.len(),
                node: new_node,
                via: *via,
                cost: new_cost + costed.cost,
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
        let steps: Vec<SearchState> = steps_reversed.into_iter().rev().collect();
        for window in steps.windows(2) {
            assert_eq!(
                (window[0].node.distance_along_way_mm - window[1].via.distance_along_way_mm).abs()
                    as u64,
                window[1].cost.distance().0 - window[0].cost.distance().0
            );
        }
        steps
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
        let costing_model = pedestrian_costing_model(1.4);
        let graph = Graph::new();
        let start = Instant::now();
        graph
            .ingest_tile(
                2625,
                5721,
                14,
                include_bytes!("../testdata/tile.pbf").to_vec(),
                include_bytes!("../testdata/tile.pbf").to_vec(),
                &costing_model,
            )
            .expect("Failed to ingest tile");
        dbg!(start.elapsed());
    }

    #[test]
    fn search_basic() {
        let costing_model = pedestrian_costing_model(1.4);
        let graph = Graph::new();
        graph
            .ingest_tile(
                2625,
                5721,
                14,
                include_bytes!("../testdata/tile.pbf").to_vec(),
                include_bytes!("../testdata/tile.pbf").to_vec(),
                &costing_model,
            )
            .expect("Failed to ingest tile");
        // approx: https://maps.earth/directions/walk/-122.315503,47.6163794/-122.3126740,47.6153470
        // ----> 325.32080857991474 meters
        let route = graph
            .search_djikstra(super::WayId(1173831634), 0, super::WayId(1172841584), 0)
            .expect("Couldn't find a route.");
        assert_eq!(route.cost.distance().mm(), 325_931);
        assert_eq!(
            route.encoded_polyline,
            "}zraHdepiV???@?@?????BCN??CPAB??A?o@?IAgC???A@?????zF?????????F??@N???L???F????A???@vE???@???B"
        );
    }

    #[test]
    fn search_fremont() {
        let costing_model = pedestrian_costing_model(1.4);
        let graph = Graph::new();
        graph
            .ingest_tile(
                2623,
                5718,
                14,
                include_bytes!("../testdata/tile2.pbf").to_vec(),
                include_bytes!("../testdata/tile2.pbf").to_vec(),
                &costing_model,
            )
            .expect("Failed to ingest tile");
        let route = graph
            .search_djikstra(super::WayId(671949014), 0, super::WayId(980366562), 0)
            .expect("Couldn't find a route.");
        dbg!(&route);
        assert_eq!(route.cost.distance().mm(), 1_996_587);
        assert_eq!(
            route.encoded_polyline,
            "{hzaHfgyiV??HY??BK??AE??m@?cBA?N????K??@?V??????M?_C???mCA??{C???WD??Q[K[IUKIOE}B?KCKISQQG??oFC??????EiAGa@Oc@Wa@WSYS???oF????M??????iB?[????G???_C@?????uE???qE?A?{A???}C?A?kE?C?qD?M@?A?M???@Q???E????U@??o@???M????????kA???I?E???q@???E???E???E???kA??E???EA???E????C?AAAA?CAA?C??????E???Q???SA??[???G???e@A????AO??EC??EAAA??@A?A?C?eA?e@?I?EACAC??????AG??ACAG??AE??M_@??E???e@???I???Q????W?[?O??I?M?C?AAA?AC????????????K?A?C???sBA??E???G??????C?aE????A?A?AAAC?C?A???A@C@A@A@???"
        );
    }
}
