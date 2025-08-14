use std::{
    collections::{BinaryHeap, HashMap},
    mem::ManuallyDrop,
    sync::Mutex,
};

use mvt_reader::feature;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WayId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WayTransition {
    distance_along_way_mm: i32,
    to_way_id: u64,
    transition_to_distance_along_way_mm: i32,
}

impl evmap::ShallowCopy for WayTransition {
    unsafe fn shallow_copy(&self) -> std::mem::ManuallyDrop<Self> {
        ManuallyDrop::new(*self)
    }
}

fn meters_to_mm_fixed(meters: f32) -> i32 {
    (meters * 1000.0) as i32
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SearchNode {
    way: WayId,
    distance_along_way_mm: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SearchCost(u64);

pub struct Graph {
    transitions_read: evmap::ReadHandle<WayId, WayTransition>,
    transitions_write: Mutex<evmap::WriteHandle<WayId, WayTransition>>,
}

impl Graph {
    pub fn new() -> Graph {
        let (tr, tw) = evmap::new();
        Graph {
            transitions_read: tr,
            transitions_write: Mutex::new(tw),
        }
    }

    pub fn ingest_tile(&self, mvt: Vec<u8>) -> anyhow::Result<()> {
        let reader = mvt_reader::Reader::new(mvt)
            .map_err(|err| anyhow::anyhow!("Could not create MVT reader {}", err))?;
        let layers = reader
            .get_layer_names()
            .map_err(|err| anyhow::anyhow!("Could not get MVT tile's layer list {}", err))?;
        if let Some((intersection_layer_id, _)) = layers
            .iter()
            .enumerate()
            .filter(|(_, layer)| layer.as_str() == "intersections")
            .next()
        {
            let features = reader.get_features(intersection_layer_id).map_err(|err| {
                anyhow::anyhow!("Could not get MVT tile's intersection features {}", err)
            })?;

            for feature in &features {
                let _props_default = HashMap::new();
                let properties = feature.properties.as_ref().unwrap_or(&_props_default);

                let from_way_id = Self::get_u64_property(properties, "way_id")?;
                let to_way_id = Self::get_u64_property(properties, "transition_to_way")?;
                let distance_along_way = Self::get_f32_property(properties, "distance_along_way")?;
                let transition_to_distance_along_way =
                    Self::get_f32_property(properties, "transition_to_distance_along_way")?;

                // // TODO: Handle restrictions.
                // let mut tags = HashMap::new();
                // for (key, value) in properties {
                //     match value {
                //         mvt_reader::feature::Value::String(value) => {
                //             tags.insert(key.clone(), value.clone())
                //         }
                //         _ => continue,
                //     };
                // }

                self.transitions_write
                    .lock()
                    .map_err(|err| anyhow::anyhow!("Failed to lock mutex: {}", err))?
                    .insert(
                        WayId(from_way_id),
                        WayTransition {
                            distance_along_way_mm: meters_to_mm_fixed(distance_along_way),
                            transition_to_distance_along_way_mm: meters_to_mm_fixed(
                                transition_to_distance_along_way,
                            ),
                            to_way_id: to_way_id,
                        },
                    );
            }
        }
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
                let mut tags = HashMap::new();
                for (key, value) in feature.properties.as_ref().unwrap_or(&HashMap::new()) {
                    match value {
                        mvt_reader::feature::Value::String(value) => {
                            tags.insert(key.clone(), value.clone())
                        }
                        _ => continue,
                    };
                }
            }
        }
        self.transitions_write
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
    ) -> Option<SearchCost> {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct SearchState {
            node: SearchNode,
            cost: SearchCost,
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

        let first_state = SearchState {
            node: SearchNode {
                way: start,
                distance_along_way_mm: distance_along_start_mm,
            },
            cost: SearchCost(0),
        };
        let mut frontier = BinaryHeap::new();
        frontier.push(first_state);
        let mut costs: HashMap<SearchNode, SearchCost> = HashMap::new();
        while let Some(state) = frontier.pop() {
            if state.node.way == end {
                let travel_distance_this_way =
                    (state.node.distance_along_way_mm - distance_along_end_mm).abs() as u64;
                return Some(SearchCost(state.cost.0 + travel_distance_this_way));
            }
            for transition in self.transitions_read.get(&state.node.way).iter().flatten() {
                let next_node = SearchNode {
                    way: WayId(transition.to_way_id),
                    distance_along_way_mm: transition.transition_to_distance_along_way_mm,
                };
                let travel_distance_this_way = (state.node.distance_along_way_mm
                    - transition.distance_along_way_mm)
                    .abs() as u64;
                let next_node_cost = SearchCost(state.cost.0 + travel_distance_this_way);
                if let Some(cost) = costs.get_mut(&next_node) {
                    if next_node_cost < *cost {
                        *cost = next_node_cost;
                        frontier.push(SearchState {
                            node: next_node,
                            cost: next_node_cost,
                        });
                    }
                } else {
                    costs.insert(next_node, next_node_cost);
                    frontier.push(SearchState {
                        node: next_node,
                        cost: next_node_cost,
                    });
                }
            }
        }
        None
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

    use super::Graph;

    #[test]
    fn ingest_tile() {
        let graph = Graph::new();
        let start = Instant::now();
        graph
            .ingest_tile(include_bytes!("../testdata/tile.pbf").to_vec())
            .expect("Failed to ingest tile");
        dbg!(start.elapsed());
    }

    #[test]
    fn search_basic() {
        let graph = Graph::new();
        graph
            .ingest_tile(include_bytes!("../testdata/tile.pbf").to_vec())
            .expect("Failed to ingest tile");
        // approx: https://maps.earth/directions/walk/-122.315503,47.6163794/-122.3126740,47.6153470
        // ----> 325.32080857991474 meters
        let cost = graph
            .search_djikstra(super::WayId(1173831634), 0, super::WayId(1172841584), 0)
            .expect("Couldn't find a route.");
        assert_eq!(cost.0, 325_931);
    }
}
