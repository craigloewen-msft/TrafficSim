use bevy::prelude::*;
use std::collections::{HashMap, VecDeque, HashSet};

use crate::intersection::{IntersectionId, IntersectionData, TrafficControlType};
use crate::road::RoadId;

/// Resource to store all road segments and intersections for pathfinding
/// This stores only IDs and entity references - actual data lives in components
#[derive(Resource, Default)]
pub struct RoadNetwork {
    pub intersections: HashMap<IntersectionId, IntersectionData>,
    /// Maps road ID to the road entity
    pub road_entities: HashMap<RoadId, Entity>,
    /// Adjacency list: intersection -> [(road_entity, road_id, next_intersection)]
    pub adjacency: HashMap<IntersectionId, Vec<(Entity, RoadId, IntersectionId)>>,
    next_intersection_id: u32,
    next_road_id: u32,
}

impl RoadNetwork {
    /// Generates a new unique intersection ID
    pub fn new_intersection_id(&mut self) -> IntersectionId {
        let id = IntersectionId(self.next_intersection_id);
        self.next_intersection_id += 1;
        id
    }

    /// Generates a new unique road ID
    pub fn new_road_id(&mut self) -> RoadId {
        let id = RoadId(self.next_road_id);
        self.next_road_id += 1;
        id
    }

    /// Adds an intersection to the network
    pub fn add_intersection(
        &mut self,
        id: IntersectionId,
        position: Vec3,
        entity: Entity,
        traffic_control: TrafficControlType,
    ) {
        self.intersections.insert(id, IntersectionData {
            position,
            entity,
            traffic_control,
        });
        self.adjacency.entry(id).or_insert_with(Vec::new);
    }

    /// Adds a road to the network and updates adjacency list
    /// The actual road data (length, speed_limit, etc.) is stored in the Road component
    pub fn add_road(
        &mut self,
        road_id: RoadId,
        road_entity: Entity,
        start_intersection: IntersectionId,
        end_intersection: IntersectionId,
    ) {
        self.road_entities.insert(road_id, road_entity);

        // Update adjacency list (bidirectional)
        self.adjacency
            .entry(start_intersection)
            .or_insert_with(Vec::new)
            .push((road_entity, road_id, end_intersection));
        
        self.adjacency
            .entry(end_intersection)
            .or_insert_with(Vec::new)
            .push((road_entity, road_id, start_intersection));
    }

    /// Finds the nearest intersection to a given position
    pub fn find_nearest_intersection(&self, position: Vec3) -> Option<IntersectionId> {
        self.intersections
            .iter()
            .min_by(|(_, a), (_, b)| {
                let dist_a = a.position.distance_squared(position);
                let dist_b = b.position.distance_squared(position);
                dist_a.partial_cmp(&dist_b).unwrap()
            })
            .map(|(id, _)| *id)
    }

    /// Gets all roads connected to an intersection
    pub fn get_connected_roads(&self, intersection_id: IntersectionId) -> Vec<(Entity, RoadId)> {
        self.adjacency
            .get(&intersection_id)
            .map(|connections| {
                connections
                    .iter()
                    .map(|(entity, road_id, _)| (*entity, *road_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gets all neighboring intersections from a given intersection
    pub fn get_neighbors(&self, intersection_id: IntersectionId) -> Vec<IntersectionId> {
        self.adjacency
            .get(&intersection_id)
            .map(|connections| connections.iter().map(|(_, _, next_id)| *next_id).collect())
            .unwrap_or_default()
    }

    /// Finds a path between two intersections (simple BFS for now)
    /// Returns a list of road entities to traverse
    pub fn find_path(&self, start: IntersectionId, end: IntersectionId) -> Option<Vec<Entity>> {
        if start == end {
            return Some(Vec::new());
        }

        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent: HashMap<IntersectionId, (IntersectionId, Entity)> = HashMap::new();

        queue.push_back(start);
        visited.insert(start);

        while let Some(current) = queue.pop_front() {
            if current == end {
                // Reconstruct path
                let mut path = Vec::new();
                let mut node = end;
                
                while let Some(&(prev, road_entity)) = parent.get(&node) {
                    path.push(road_entity);
                    node = prev;
                }
                
                path.reverse();
                return Some(path);
            }

            if let Some(connections) = self.adjacency.get(&current) {
                for &(road_entity, _, next_id) in connections {
                    if !visited.contains(&next_id) {
                        visited.insert(next_id);
                        parent.insert(next_id, (current, road_entity));
                        queue.push_back(next_id);
                    }
                }
            }
        }

        None // No path found
    }
}
