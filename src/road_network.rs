use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::intersection::IntersectionEntity;
use crate::road::RoadEntity;

/// Resource to store the road network graph for pathfinding
/// Actual road and intersection data lives in their respective components
#[derive(Resource, Default)]
pub struct RoadNetwork {
    /// Graph structure: intersection -> [(road, next_intersection)]
    /// This is a bidirectional graph where each intersection knows its connected roads and neighbors
    pub adjacency: HashMap<IntersectionEntity, Vec<(RoadEntity, IntersectionEntity)>>,
}

impl RoadNetwork {
    /// Adds an intersection to the network graph
    pub fn add_intersection(&mut self, intersection_entity: IntersectionEntity) {
        self.adjacency
            .entry(intersection_entity)
            .or_default();
    }

    /// Adds a road to the network and updates the graph adjacency
    pub fn add_road(
        &mut self,
        road_entity: RoadEntity,
        start_intersection_entity: IntersectionEntity,
        end_intersection_entity: IntersectionEntity,
    ) {
        // Update adjacency list (bidirectional)
        self.adjacency
            .entry(start_intersection_entity)
            .or_default()
            .push((road_entity, end_intersection_entity));

        self.adjacency
            .entry(end_intersection_entity)
            .or_default()
            .push((road_entity, start_intersection_entity));
    }

    /// Finds the road entity connecting two intersection entities
    pub fn find_road_between(
        &self,
        from_intersection_entity: IntersectionEntity,
        to_intersection_entity: IntersectionEntity,
    ) -> anyhow::Result<RoadEntity> {
        // Look up the road in the adjacency list
        let connections = self
            .adjacency
            .get(&from_intersection_entity)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Intersection {:?} not found in road network",
                    from_intersection_entity
                )
            })?;

        connections
            .iter()
            .find(|(_, next_intersection_entity)| {
                *next_intersection_entity == to_intersection_entity
            })
            .map(|(road_entity, _)| *road_entity)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No road found connecting intersection {:?} to intersection {:?}",
                    from_intersection_entity,
                    to_intersection_entity
                )
            })
    }

    /// Finds a path between two intersections (simple BFS for now)
    /// Returns a list of intersection entities to traverse (excluding start, including end)
    pub fn find_path(
        &self,
        start_intersection_entity: IntersectionEntity,
        end_intersection_entity: IntersectionEntity,
    ) -> Option<Vec<IntersectionEntity>> {
        if start_intersection_entity == end_intersection_entity {
            return Some(vec![]);
        }

        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent: HashMap<IntersectionEntity, IntersectionEntity> = HashMap::new();

        queue.push_back(start_intersection_entity);
        visited.insert(start_intersection_entity);

        while let Some(current_intersection_entity) = queue.pop_front() {
            if current_intersection_entity == end_intersection_entity {
                // Reconstruct path (excluding start node)
                let mut path = Vec::new();
                let mut node_intersection_entity = end_intersection_entity;

                while let Some(&prev_intersection_entity) = parent.get(&node_intersection_entity) {
                    path.push(node_intersection_entity);
                    node_intersection_entity = prev_intersection_entity;
                }
                // Don't push start_intersection_entity

                path.reverse();
                return Some(path);
            }

            if let Some(connections) = self.adjacency.get(&current_intersection_entity) {
                for &(_, next_intersection_entity) in connections {
                    if !visited.contains(&next_intersection_entity) {
                        visited.insert(next_intersection_entity);
                        parent.insert(next_intersection_entity, current_intersection_entity);
                        queue.push_back(next_intersection_entity);
                    }
                }
            }
        }

        None // No path found
    }
}
