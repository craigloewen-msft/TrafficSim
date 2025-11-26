//! Road network graph for pathfinding
//!
//! Standalone implementation that doesn't depend on Bevy.

use anyhow::{Context, Result};
use ordered_float::OrderedFloat;
use petgraph::algo::astar;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::ops::Bound;

use super::types::{CarId, IntersectionId, Position, RoadId, SimRoad};

/// Edge data for the road network graph
#[derive(Debug, Clone, Copy)]
pub struct RoadEdge {
    pub road_id: RoadId,
    pub weight: u32, // Road length scaled for integer weights
}

impl RoadEdge {
    pub fn from_road(road: &SimRoad) -> Self {
        // Convert road length to integer weight (scaled by 100 to preserve precision)
        let weight = (road.length * 100.0) as u32;
        Self {
            road_id: road.id,
            weight: weight.max(1), // Ensure minimum weight of 1
        }
    }
}

/// Standalone road network graph for pathfinding
/// This doesn't depend on Bevy's ECS system
#[derive(Default)]
#[allow(dead_code)]
pub struct SimRoadNetwork {
    /// The underlying petgraph directed graph (one-way roads)
    graph: DiGraph<IntersectionId, RoadEdge>,

    /// Maps intersection IDs to their node indices in the graph
    intersection_to_node: HashMap<IntersectionId, NodeIndex>,

    /// Maps node indices back to intersection IDs
    node_to_intersection: HashMap<NodeIndex, IntersectionId>,

    /// Cached path results
    path_cache: HashMap<IntersectionId, HashMap<IntersectionId, Vec<IntersectionId>>>,

    /// Maps road IDs to lists of (distance, car_id) tuples for traffic detection
    cars_on_roads: HashMap<RoadId, BTreeMap<OrderedFloat<f32>, CarId>>,

    /// Storage for road data
    roads: HashMap<RoadId, SimRoad>,

    /// Storage for intersection positions
    intersection_positions: HashMap<IntersectionId, Position>,
}

impl SimRoadNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an intersection to the network graph
    pub fn add_intersection(&mut self, intersection_id: IntersectionId, position: Position) {
        if self.intersection_to_node.contains_key(&intersection_id) {
            return;
        }

        let node_index = self.graph.add_node(intersection_id);
        self.intersection_to_node
            .insert(intersection_id, node_index);
        self.node_to_intersection
            .insert(node_index, intersection_id);
        self.intersection_positions
            .insert(intersection_id, position);
        self.path_cache.clear();
    }

    /// Gets the position of an intersection
    pub fn get_intersection_position(&self, intersection_id: IntersectionId) -> Option<&Position> {
        self.intersection_positions.get(&intersection_id)
    }

    /// Adds a road to the network and updates the graph adjacency
    pub fn add_road(&mut self, road: SimRoad) {
        let start_id = road.start_intersection;
        let end_id = road.end_intersection;

        // Ensure both intersections exist (they should already, but just in case)
        if !self.intersection_to_node.contains_key(&start_id) {
            self.add_intersection(start_id, Position::default());
        }
        if !self.intersection_to_node.contains_key(&end_id) {
            self.add_intersection(end_id, Position::default());
        }

        let start_node = self.intersection_to_node[&start_id];
        let end_node = self.intersection_to_node[&end_id];

        let edge_data = RoadEdge::from_road(&road);
        self.graph.add_edge(start_node, end_node, edge_data);

        // Store the road
        self.roads.insert(road.id, road);

        self.path_cache.clear();
    }

    /// Gets a road by ID
    pub fn get_road(&self, road_id: RoadId) -> Option<&SimRoad> {
        self.roads.get(&road_id)
    }

    /// Finds the road connecting two intersections
    pub fn find_road_between(
        &self,
        from_intersection: IntersectionId,
        to_intersection: IntersectionId,
    ) -> Result<RoadId> {
        let from_node = self
            .intersection_to_node
            .get(&from_intersection)
            .ok_or_else(|| anyhow::anyhow!("Intersection {:?} not found", from_intersection))?;

        let to_node = self
            .intersection_to_node
            .get(&to_intersection)
            .ok_or_else(|| anyhow::anyhow!("Intersection {:?} not found", to_intersection))?;

        self.graph
            .edges(*from_node)
            .find(|edge| edge.target() == *to_node)
            .map(|edge| edge.weight().road_id)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No road found connecting {:?} to {:?}",
                    from_intersection,
                    to_intersection
                )
            })
    }

    /// Finds a path between two intersections using A* (Dijkstra with null heuristic)
    pub fn find_path(
        &mut self,
        start: IntersectionId,
        end: IntersectionId,
    ) -> Option<Vec<IntersectionId>> {
        if start == end {
            return Some(vec![]);
        }

        // Check cache first
        if let Some(cached_paths) = self.path_cache.get(&start) {
            if let Some(path) = cached_paths.get(&end) {
                return Some(path.clone());
            }
        }

        let start_node = self.intersection_to_node.get(&start)?;
        let end_node = self.intersection_to_node.get(&end)?;

        let result = astar(
            &self.graph,
            *start_node,
            |node| node == *end_node,
            |edge| edge.weight().weight,
            |_| 0, // Null heuristic = Dijkstra
        )?;

        let (_, node_path) = result;

        // Convert node indices to intersection IDs, excluding the start node
        let path: Vec<IntersectionId> = node_path
            .iter()
            .skip(1)
            .filter_map(|node_idx| self.node_to_intersection.get(node_idx).copied())
            .collect();

        // Cache the result
        self.path_cache
            .entry(start)
            .or_default()
            .insert(end, path.clone());

        Some(path)
    }

    /// Gets all intersection IDs in the network
    #[allow(dead_code)]
    pub fn get_all_intersections(&self) -> Vec<IntersectionId> {
        self.intersection_to_node.keys().copied().collect()
    }

    /// Gets all roads connected to a specific intersection
    pub fn get_connected_roads(
        &self,
        intersection_id: IntersectionId,
    ) -> Option<Vec<(RoadId, IntersectionId)>> {
        let node_index = self.intersection_to_node.get(&intersection_id)?;

        let connections: Vec<_> = self
            .graph
            .edges(*node_index)
            .map(|edge| {
                let road_id = edge.weight().road_id;
                let next_intersection = self.node_to_intersection[&edge.target()];
                (road_id, next_intersection)
            })
            .collect();

        Some(connections)
    }

    /// Update a car's position on a road for traffic tracking
    pub fn update_car_road_position(
        &mut self,
        car_id: CarId,
        road_id: RoadId,
        distance: OrderedFloat<f32>,
        remove: bool,
        prev_road_id: Option<RoadId>,
        prev_distance: OrderedFloat<f32>,
    ) -> Result<()> {
        if remove {
            self.cars_on_roads
                .get_mut(&road_id)
                .context("Couldn't find road list to delete")?
                .retain(|_distance, visitor_id| *visitor_id != car_id);
        } else {
            // Remove from old position
            if let Some(prev_road) = prev_road_id {
                if let Some(car_map) = self.cars_on_roads.get_mut(&prev_road) {
                    car_map.remove(&prev_distance);
                }
            }

            // Insert at new position
            let car_map = self.cars_on_roads.entry(road_id).or_default();
            car_map.insert(distance, car_id);
        }

        Ok(())
    }

    /// Find the car directly ahead on the same road
    pub fn find_car_ahead_on_road(
        &self,
        road_id: RoadId,
        current_distance: &OrderedFloat<f32>,
    ) -> Result<Option<(&OrderedFloat<f32>, CarId)>> {
        let car_map = self
            .cars_on_roads
            .get(&road_id)
            .context("Road has no car list")?;

        Ok(car_map
            .range((Bound::Excluded(current_distance), Bound::Unbounded))
            .next()
            .map(|(distance, car)| (distance, *car)))
    }

    /// Get number of roads
    pub fn road_count(&self) -> usize {
        self.roads.len()
    }

    /// Get number of intersections
    pub fn intersection_count(&self) -> usize {
        self.intersection_to_node.len()
    }

    /// Get all roads (used by UI for rendering)
    #[cfg_attr(not(feature = "ui"), allow(dead_code))]
    pub fn get_all_roads(&self) -> impl Iterator<Item = (&RoadId, &SimRoad)> {
        self.roads.iter()
    }

    /// Get all roads
    pub fn roads(&self) -> &HashMap<RoadId, SimRoad> {
        &self.roads
    }

    /// Get all intersection positions
    pub fn intersection_positions(&self) -> &HashMap<IntersectionId, Position> {
        &self.intersection_positions
    }

    /// Remove a road from the network
    /// Returns the cars that were on the road
    pub fn remove_road(&mut self, road_id: RoadId) -> Result<Vec<CarId>> {
        let road = self.roads.remove(&road_id).context("Road not found")?;

        let start_node = self
            .intersection_to_node
            .get(&road.start_intersection)
            .context("Start intersection not found")?;

        let end_node = self
            .intersection_to_node
            .get(&road.end_intersection)
            .context("End intersection not found")?;

        // Find and remove the edge
        let edge_to_remove = self
            .graph
            .edges(*start_node)
            .find(|edge| edge.target() == *end_node && edge.weight().road_id == road_id)
            .map(|edge| edge.id());

        if let Some(edge_id) = edge_to_remove {
            self.graph.remove_edge(edge_id);
        }

        // Get cars that were on this road
        let cars = self
            .cars_on_roads
            .remove(&road_id)
            .map(|car_map| car_map.values().copied().collect())
            .unwrap_or_default();

        self.path_cache.clear();

        Ok(cars)
    }

    /// Remove an intersection and all connected roads
    /// Returns (removed_road_ids, cars_on_removed_roads)
    pub fn remove_intersection(
        &mut self,
        intersection_id: IntersectionId,
    ) -> Result<(Vec<RoadId>, Vec<CarId>)> {
        let node_index = self
            .intersection_to_node
            .remove(&intersection_id)
            .context("Intersection not found")?;

        self.node_to_intersection.remove(&node_index);
        self.intersection_positions.remove(&intersection_id);

        // Find all roads connected to this intersection
        let roads_to_remove: Vec<RoadId> = self
            .roads
            .iter()
            .filter(|(_, road)| {
                road.start_intersection == intersection_id
                    || road.end_intersection == intersection_id
            })
            .map(|(id, _)| *id)
            .collect();

        // Remove roads and collect affected cars
        let mut affected_cars = Vec::new();
        for road_id in &roads_to_remove {
            self.roads.remove(road_id);
            if let Some(car_map) = self.cars_on_roads.remove(road_id) {
                affected_cars.extend(car_map.values().copied());
            }
        }

        // Remove the node from the graph (this also removes all edges)
        self.graph.remove_node(node_index);

        self.path_cache.clear();

        Ok((roads_to_remove, affected_cars))
    }

    /// Get all roads starting or ending at an intersection
    pub fn get_roads_at_intersection(&self, intersection_id: IntersectionId) -> Vec<RoadId> {
        self.roads
            .iter()
            .filter(|(_, road)| {
                road.start_intersection == intersection_id
                    || road.end_intersection == intersection_id
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get all cars currently on a specific road
    pub fn get_cars_on_road(&self, road_id: RoadId) -> Vec<CarId> {
        self.cars_on_roads
            .get(&road_id)
            .map(|car_map| car_map.values().copied().collect())
            .unwrap_or_default()
    }

    /// Remove a car from road tracking
    pub fn remove_car_from_tracking(&mut self, car_id: CarId) {
        for car_map in self.cars_on_roads.values_mut() {
            car_map.retain(|_, id| *id != car_id);
        }
    }

    /// Check if an intersection has any connected roads
    pub fn intersection_has_roads(&self, intersection_id: IntersectionId) -> bool {
        self.roads.values().any(|road| {
            road.start_intersection == intersection_id || road.end_intersection == intersection_id
        })
    }

    /// Find the closest intersection to a given position
    pub fn find_closest_intersection(&self, position: &Position) -> Option<IntersectionId> {
        self.intersection_positions
            .iter()
            .min_by(|(_, pos_a), (_, pos_b)| {
                let dist_a = position.distance(pos_a);
                let dist_b = position.distance(pos_b);
                dist_a
                    .partial_cmp(&dist_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(id, _)| *id)
    }

    /// Find the closest point on any road to a given position
    /// Returns (road_id, closest_position, distance_along_road, total_road_length)
    pub fn find_closest_point_on_road(
        &self,
        position: &Position,
    ) -> Option<(RoadId, Position, f32, f32)> {
        let mut closest: Option<(RoadId, Position, f32, f32, f32)> = None;

        for (road_id, road) in &self.roads {
            let start_pos = self.intersection_positions.get(&road.start_intersection)?;
            let end_pos = self.intersection_positions.get(&road.end_intersection)?;

            // Calculate projection of position onto road line
            let road_vec_x = end_pos.x - start_pos.x;
            let road_vec_z = end_pos.z - start_pos.z;
            let road_length_sq = road_vec_x * road_vec_x + road_vec_z * road_vec_z;

            if road_length_sq < 0.001 {
                continue;
            }

            let pos_vec_x = position.x - start_pos.x;
            let pos_vec_z = position.z - start_pos.z;

            let t = ((pos_vec_x * road_vec_x + pos_vec_z * road_vec_z) / road_length_sq)
                .clamp(0.0, 1.0);

            let closest_point = Position::new(
                start_pos.x + t * road_vec_x,
                start_pos.y,
                start_pos.z + t * road_vec_z,
            );

            let distance = position.distance(&closest_point);
            let distance_along_road = t * road.length;

            if closest.is_none() || distance < closest.as_ref().unwrap().4 {
                closest = Some((
                    *road_id,
                    closest_point,
                    distance_along_road,
                    road.length,
                    distance,
                ));
            }
        }

        closest.map(|(road_id, pos, dist_along, length, _)| (road_id, pos, dist_along, length))
    }
}
