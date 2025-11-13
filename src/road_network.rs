use bevy::prelude::*;
use std::collections::{HashMap, VecDeque, HashSet};

use crate::intersection::{IntersectionData, TrafficControlType, IntersectionEntity};
use crate::road::RoadId;

/// Resource to store all road segments and intersections for pathfinding
/// This stores only IDs and entity references - actual data lives in components
#[derive(Resource, Default)]
pub struct RoadNetwork {
    pub intersections: HashMap<Entity, IntersectionData>,
    /// Maps road ID to the road entity
    pub road_entities: HashMap<RoadId, Entity>,
    /// Adjacency list: intersection entity -> [(road_entity, road_id, next_intersection_entity)]
    pub adjacency: HashMap<Entity, Vec<(Entity, RoadId, Entity)>>,
    next_road_id: u32,
}

impl RoadNetwork {
    /// Generates a new unique road ID
    pub fn new_road_id(&mut self) -> RoadId {
        let id = RoadId(self.next_road_id);
        self.next_road_id += 1;
        id
    }

    /// Adds an intersection to the network
    pub fn add_intersection(
        &mut self,
        entity: Entity,
        position: Vec3,
        traffic_control: TrafficControlType,
    ) {
        self.intersections.insert(entity, IntersectionData {
            position,
            entity,
            traffic_control,
        });
        self.adjacency.entry(entity).or_insert_with(Vec::new);
    }

    /// Adds a road to the network and updates adjacency list
    /// The actual road data (length, speed_limit, etc.) is stored in the Road component
    pub fn add_road(
        &mut self,
        road_id: RoadId,
        road_entity: Entity,
        start_intersection: Entity,
        end_intersection: Entity,
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
    pub fn find_nearest_intersection(&self, position: Vec3) -> Option<Entity> {
        self.intersections
            .iter()
            .min_by(|(_, a), (_, b)| {
                let dist_a = a.position.distance_squared(position);
                let dist_b = b.position.distance_squared(position);
                dist_a.partial_cmp(&dist_b).unwrap()
            })
            .map(|(entity, _)| *entity)
    }

    /// Gets all roads connected to an intersection
    pub fn get_connected_roads(&self, intersection_entity: Entity) -> Vec<(Entity, RoadId)> {
        self.adjacency
            .get(&intersection_entity)
            .map(|connections| {
                connections
                    .iter()
                    .map(|(entity, road_id, _)| (*entity, *road_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gets all neighboring intersections from a given intersection
    pub fn get_neighbors(&self, intersection_entity: Entity) -> Vec<Entity> {
        self.adjacency
            .get(&intersection_entity)
            .map(|connections| connections.iter().map(|(_, _, next_entity)| *next_entity).collect())
            .unwrap_or_default()
    }

    /// Finds the road entity connecting two intersection entities
    pub fn find_road_between(&self, from: Entity, to: Entity) -> Option<Entity> {
        // Look up the road in the adjacency list
        self.adjacency
            .get(&from)?
            .iter()
            .find(|(_, _, next_entity)| *next_entity == to)
            .map(|(road_entity, _, _)| *road_entity)
    }

    /// Finds a path between two intersections (simple BFS for now)
    /// Returns a list of intersection entities to traverse (including start and end)
    pub fn find_path(&self, start: Entity, end: Entity) -> Option<Vec<IntersectionEntity>> {
        if start == end {
            return Some(vec![IntersectionEntity(start)]);
        }

        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent: HashMap<Entity, Entity> = HashMap::new();

        queue.push_back(start);
        visited.insert(start);

        while let Some(current) = queue.pop_front() {
            if current == end {
                // Reconstruct path
                let mut path = Vec::new();
                let mut node = end;
                
                while let Some(&prev) = parent.get(&node) {
                    path.push(IntersectionEntity(node));
                    node = prev;
                }
                path.push(IntersectionEntity(start));
                
                path.reverse();
                return Some(path);
            }

            if let Some(connections) = self.adjacency.get(&current) {
                for &(_, _, next_entity) in connections {
                    if !visited.contains(&next_entity) {
                        visited.insert(next_entity);
                        parent.insert(next_entity, current);
                        queue.push_back(next_entity);
                    }
                }
            }
        }

        None // No path found
    }
}
