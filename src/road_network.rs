use bevy::prelude::*;
use petgraph::algo::astar;
use petgraph::graph::{NodeIndex, DiGraph};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

use crate::car::CarEntity;
use crate::intersection::IntersectionEntity;
use crate::road::{Road, RoadEntity};

/// Edge data for the road network graph
#[derive(Debug, Clone, Copy)]
pub struct RoadEdge {
    pub road_entity: RoadEntity,
    pub weight: u32, // Road length scaled for integer weights
}

impl From<(RoadEntity, &Road)> for RoadEdge {
    fn from((road_entity, road): (RoadEntity, &Road)) -> Self {
        // Convert road length to integer weight (scaled by 100 to preserve precision)
        let weight = (road.length * 100.0) as u32;
        
        Self {
            road_entity,
            weight: weight.max(1), // Ensure minimum weight of 1
        }
    }
}

impl RoadEdge {
    /// Creates a RoadEdge with a default weight
    /// Useful when road data is not available
    pub fn with_default_weight(road_entity: RoadEntity) -> Self {
        Self {
            road_entity,
            weight: 1,
        }
    }
}

/// Resource to store the road network graph for pathfinding
/// Actual road and intersection data lives in their respective components
#[derive(Resource)]
pub struct RoadNetwork {
    /// The underlying petgraph directed graph (one-way roads)
    graph: DiGraph<IntersectionEntity, RoadEdge>,
    
    /// Maps intersection entities to their node indices in the graph
    intersection_to_node: HashMap<IntersectionEntity, NodeIndex>,
    
    /// Maps node indices back to intersection entities
    node_to_intersection: HashMap<NodeIndex, IntersectionEntity>,
    
    /// Cached Dijkstra results: maps from source intersection to (distances, predecessors)
    /// This is invalidated whenever the graph structure changes
    path_cache: HashMap<IntersectionEntity, HashMap<IntersectionEntity, Vec<IntersectionEntity>>>,
    
    /// Maps road entities to lists of (car_entity, progress) tuples for traffic detection
    /// This is private and should only be accessed through public methods
    cars_on_roads: HashMap<Entity, Vec<(CarEntity, f32)>>,
}

impl Default for RoadNetwork {
    fn default() -> Self {
        Self {
            graph: DiGraph::new(),
            intersection_to_node: HashMap::new(),
            node_to_intersection: HashMap::new(),
            path_cache: HashMap::new(),
            cars_on_roads: HashMap::new(),
        }
    }
}

impl RoadNetwork {
    /// Adds an intersection to the network graph
    pub fn add_intersection(&mut self, intersection_entity: IntersectionEntity) {
        // Check if already exists
        if self.intersection_to_node.contains_key(&intersection_entity) {
            return;
        }
        
        let node_index = self.graph.add_node(intersection_entity);
        self.intersection_to_node.insert(intersection_entity, node_index);
        self.node_to_intersection.insert(node_index, intersection_entity);
        
        // Invalidate path cache when graph structure changes
        self.path_cache.clear();
    }

    /// Adds a road to the network and updates the graph adjacency
    /// Uses the Road component to calculate edge weight from road length
    pub fn add_road(
        &mut self,
        road_entity: RoadEntity,
        road: &Road,
    ) {
        let start_intersection_entity = road.start_intersection_entity;
        let end_intersection_entity = road.end_intersection_entity;
        
        // Ensure both intersections exist in the graph
        if !self.intersection_to_node.contains_key(&start_intersection_entity) {
            self.add_intersection(start_intersection_entity);
        }
        if !self.intersection_to_node.contains_key(&end_intersection_entity) {
            self.add_intersection(end_intersection_entity);
        }
        
        let start_node = self.intersection_to_node[&start_intersection_entity];
        let end_node = self.intersection_to_node[&end_intersection_entity];
        
        // Use From trait to convert Road to RoadEdge
        let edge_data = RoadEdge::from((road_entity, road));
        
        self.graph.add_edge(start_node, end_node, edge_data);
        
        // Invalidate path cache when graph structure changes
        self.path_cache.clear();
    }

    /// Finds the road entity connecting two intersection entities
    pub fn find_road_between(
        &self,
        from_intersection_entity: IntersectionEntity,
        to_intersection_entity: IntersectionEntity,
    ) -> anyhow::Result<RoadEntity> {
        let from_node = self
            .intersection_to_node
            .get(&from_intersection_entity)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Intersection {:?} not found in road network",
                    from_intersection_entity
                )
            })?;
            
        let to_node = self
            .intersection_to_node
            .get(&to_intersection_entity)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Intersection {:?} not found in road network",
                    to_intersection_entity
                )
            })?;
        
        self.graph
            .edges(*from_node)
            .find(|edge| edge.target() == *to_node)
            .map(|edge| edge.weight().road_entity)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No road found connecting intersection {:?} to intersection {:?}",
                    from_intersection_entity,
                    to_intersection_entity
                )
            })
    }

    /// Finds a path between two intersections using Dijkstra's algorithm (via A* with null heuristic)
    /// Returns a list of intersection entities to traverse (excluding start, including end)
    /// Results are cached until the graph structure changes
    pub fn find_path(
        &mut self,
        start_intersection_entity: IntersectionEntity,
        end_intersection_entity: IntersectionEntity,
    ) -> Option<Vec<IntersectionEntity>> {
        if start_intersection_entity == end_intersection_entity {
            return Some(vec![]);
        }

        // Check cache first
        if let Some(cached_paths) = self.path_cache.get(&start_intersection_entity) {
            if let Some(path) = cached_paths.get(&end_intersection_entity) {
                return Some(path.clone());
            }
        }

        let start_node = self.intersection_to_node.get(&start_intersection_entity)?;
        let end_node = self.intersection_to_node.get(&end_intersection_entity)?;

        // Use A* with a null heuristic (equivalent to Dijkstra) which returns the path directly
        let result = astar(
            &self.graph,
            *start_node,
            |node| node == *end_node,
            |edge| edge.weight().weight,
            |_| 0, // Null heuristic makes this equivalent to Dijkstra
        )?;

        // astar returns (cost, Vec<NodeIndex>) where the Vec is the full path including start and end
        let (_, node_path) = result;
        
        // Convert node indices to intersection entities, excluding the start node
        let path: Vec<IntersectionEntity> = node_path
            .iter()
            .skip(1) // Skip the start node
            .filter_map(|node_idx| self.node_to_intersection.get(node_idx).copied())
            .collect();
        
        // Cache the result
        self.path_cache
            .entry(start_intersection_entity)
            .or_insert_with(HashMap::new)
            .insert(end_intersection_entity, path.clone());

        Some(path)
    }
    
    /// Removes an intersection from the network
    /// This will also remove all roads connected to it and invalidate the cache
    pub fn remove_intersection(&mut self, intersection_entity: IntersectionEntity) -> bool {
        if let Some(node_index) = self.intersection_to_node.remove(&intersection_entity) {
            self.node_to_intersection.remove(&node_index);
            self.graph.remove_node(node_index);
            self.path_cache.clear();
            true
        } else {
            false
        }
    }
    
    /// Gets all intersection entities in the network
    pub fn get_all_intersections(&self) -> Vec<IntersectionEntity> {
        self.intersection_to_node.keys().copied().collect()
    }
    
    /// Gets all roads connected to a specific intersection
    /// Returns a list of (road_entity, next_intersection) pairs
    pub fn get_connected_roads(
        &self,
        intersection_entity: IntersectionEntity,
    ) -> Option<Vec<(RoadEntity, IntersectionEntity)>> {
        let node_index = self.intersection_to_node.get(&intersection_entity)?;
        
        let connections: Vec<_> = self.graph
            .edges(*node_index)
            .map(|edge| {
                let road_entity = edge.weight().road_entity;
                let next_intersection = self.node_to_intersection[&edge.target()];
                (road_entity, next_intersection)
            })
            .collect();
        
        Some(connections)
    }
    
    // Car tracking methods for traffic detection
    
    /// Clear all car tracking data (called at start of each frame)
    pub fn clear_car_tracking(&mut self) {
        self.cars_on_roads.clear();
    }

    /// Register a car on a road for traffic tracking
    /// This should be called once per frame for each car before querying traffic ahead
    pub fn register_car_on_road(&mut self, road_entity: RoadEntity, car_entity: CarEntity, progress: f32) {
        self.cars_on_roads
            .entry(road_entity.0)
            .or_insert_with(Vec::new)
            .push((car_entity, progress));
    }

    /// Find the car directly ahead on the same road
    /// Returns Some((car_entity, progress)) of the nearest car ahead, or None if no car is ahead
    /// 
    /// # Arguments
    /// * `road_entity` - The road to check for cars
    /// * `current_car` - The car entity making the query (to exclude from results)
    /// * `current_progress` - The current progress (0.0-1.0) of the querying car along the road
    pub fn find_car_ahead_on_road(
        &self, 
        road_entity: RoadEntity, 
        current_car: CarEntity, 
        current_progress: f32
    ) -> Option<(CarEntity, f32)> {
        let cars_on_road = self.cars_on_roads.get(&road_entity.0)?;
        
        // Find all cars ahead (with higher progress values)
        let mut cars_ahead: Vec<(CarEntity, f32)> = cars_on_road
            .iter()
            .filter(|(car, progress)| car.0 != current_car.0 && *progress > current_progress)
            .copied()
            .collect();
        
        // Sort by progress and return the closest one (smallest progress value that's still ahead)
        cars_ahead.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        cars_ahead.first().copied()
    }
}
