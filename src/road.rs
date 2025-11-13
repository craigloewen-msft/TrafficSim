use bevy::prelude::*;

use crate::intersection::{Intersection, TrafficControlType};
use crate::road_network::RoadNetwork;

/// Unique identifier for roads
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoadId(pub u32);

/// Component that marks an entity as a road segment
#[derive(Component, Debug)]
pub struct Road {
    pub id: RoadId,
    pub start_intersection: Entity,
    pub end_intersection: Entity,
    pub lane_count: u32,
    pub speed_limit: f32, // m/s
    pub angle: f32, // Rotation angle in radians (Y-axis rotation)
}

impl Road {
    /// Creates a new road segment between two intersections
    pub fn new(
        id: RoadId,
        start_intersection: Entity,
        end_intersection: Entity,
        start_pos: Vec3,
        end_pos: Vec3,
    ) -> Self {
        let direction = (end_pos - start_pos).normalize();
        let angle = direction.x.atan2(direction.z);
        
        Self {
            id,
            start_intersection,
            end_intersection,
            lane_count: 2, // Default 2 lanes
            speed_limit: 13.4, // Default ~30 mph in m/s
            angle,
        }
    }

    /// Spawns the road entity with all necessary components
    pub fn spawn(
        &self,
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<StandardMaterial>>,
        start_pos: Vec3,
        end_pos: Vec3,
    ) -> Entity {
        const ROAD_WIDTH: f32 = 0.4;
        const ROAD_HEIGHT: f32 = 0.02;
        let road_color = Color::srgb(0.2, 0.2, 0.2);

        // Calculate road properties
        let length = start_pos.distance(end_pos);
        let midpoint = (start_pos + end_pos) / 2.0;
        
        // Use the stored angle for rotation
        let rotation = Quat::from_rotation_y(self.angle);

        // Spawn road segment
        commands.spawn((
            Road {
                id: self.id,
                start_intersection: self.start_intersection,
                end_intersection: self.end_intersection,
                lane_count: self.lane_count,
                speed_limit: self.speed_limit,
                angle: self.angle,
            },
            Mesh3d(meshes.add(Cuboid::new(ROAD_WIDTH, ROAD_HEIGHT, length))),
            MeshMaterial3d(materials.add(road_color)),
            Transform::from_translation(Vec3::new(
                midpoint.x,
                ROAD_HEIGHT / 2.0,
                midpoint.z,
            ))
            .with_rotation(rotation),
        )).id()
    }
}

/// Helper function to spawn a road between two positions
/// This will find or create intersections at the given positions
pub fn spawn_road_at_positions(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &mut ResMut<RoadNetwork>,
    start_pos: Vec3,
    end_pos: Vec3,
) {
    const INTERSECTION_SNAP_DISTANCE: f32 = 0.1;
    
    // Find or create start intersection
    let start_id = road_network
        .find_nearest_intersection(start_pos)
        .filter(|&id| {
            road_network.intersections[&id]
                .position
                .distance(start_pos) < INTERSECTION_SNAP_DISTANCE
        })
        .unwrap_or_else(|| {
            let id = road_network.new_intersection_id();
            let intersection = Intersection::new(id, start_pos, TrafficControlType::None);
            let entity = intersection.spawn(commands, meshes, materials);
            road_network.add_intersection(id, start_pos, entity, TrafficControlType::None);
            id
        });
    
    // Find or create end intersection
    let end_id = road_network
        .find_nearest_intersection(end_pos)
        .filter(|&id| {
            road_network.intersections[&id]
                .position
                .distance(end_pos) < INTERSECTION_SNAP_DISTANCE
        })
        .unwrap_or_else(|| {
            let id = road_network.new_intersection_id();
            let intersection = Intersection::new(id, end_pos, TrafficControlType::None);
            let entity = intersection.spawn(commands, meshes, materials);
            road_network.add_intersection(id, end_pos, entity, TrafficControlType::None);
            id
        });
    
    let start_entity = road_network.intersections[&start_id].entity;
    let end_entity = road_network.intersections[&end_id].entity;
    
    // Create and spawn road
    let road_id = road_network.new_road_id();
    let road = Road::new(road_id, start_entity, end_entity, start_pos, end_pos);
    let road_entity = road.spawn(commands, meshes, materials, start_pos, end_pos);
    
    // Add to network (no need to pass length/speed_limit, they're in the Road component)
    road_network.add_road(road_id, road_entity, start_id, end_id);
}

/// System to spawn roads connecting houses
pub fn spawn_roads(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut road_network: ResMut<RoadNetwork>,
) {
    // Define house positions (should match house.rs)
    let house_positions = vec![
        Vec3::new(-8.0, 0.0, -8.0),
        Vec3::new(8.0, 0.0, -8.0),
        Vec3::new(-8.0, 0.0, 8.0),
        Vec3::new(8.0, 0.0, 8.0),
    ];

    // First, create all intersections at house positions
    let mut intersection_entities = Vec::new();
    let mut intersection_ids = Vec::new();
    
    for position in &house_positions {
        let id = road_network.new_intersection_id();
        let intersection = Intersection::new(id, *position, TrafficControlType::StopSign);
        let entity = intersection.spawn(&mut commands, &mut meshes, &mut materials);
        
        road_network.add_intersection(id, *position, entity, TrafficControlType::StopSign);
        
        intersection_entities.push(entity);
        intersection_ids.push(id);
    }

    // Create road connections between intersections
    let road_connections = vec![
        (0, 1), // Left to Right (top)
        (2, 3), // Left to Right (bottom)
        (0, 2), // Top to Bottom (left)
        (1, 3), // Top to Bottom (right)
    ];

    for (start_idx, end_idx) in road_connections {
        let start_pos = house_positions[start_idx];
        let end_pos = house_positions[end_idx];
        let start_entity = intersection_entities[start_idx];
        let end_entity = intersection_entities[end_idx];
        let start_id = intersection_ids[start_idx];
        let end_id = intersection_ids[end_idx];
        
        // Create and spawn road
        let road_id = road_network.new_road_id();
        let road = Road::new(road_id, start_entity, end_entity, start_pos, end_pos);
        let road_entity = road.spawn(&mut commands, &mut meshes, &mut materials, start_pos, end_pos);
        
        // Add to network (no need to pass length/speed_limit, they're in the Road component)
        road_network.add_road(road_id, road_entity, start_id, end_id);
    }
}

/// Plugin to register all road-related systems
pub struct RoadPlugin;

impl Plugin for RoadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoadNetwork>()
            .add_systems(Startup, spawn_roads);
    }
}
