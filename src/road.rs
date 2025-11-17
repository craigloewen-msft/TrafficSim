use anyhow::Result;
use bevy::prelude::*;

use crate::intersection::{IntersectionEntity};
use crate::road_network::RoadNetwork;

// Import the spawn helper function
use crate::intersection::spawn_intersection;

/// Wrapper type for road entities to provide type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoadEntity(pub Entity);

/// Component that marks an entity as a road segment
#[derive(Component, Debug, Clone, Copy)]
pub struct Road {
    pub start_intersection_entity: IntersectionEntity,
    pub end_intersection_entity: IntersectionEntity,
    pub length: f32,      // Length of the road in world units
    pub angle: f32,       // Rotation angle in radians (Y-axis rotation)
    // pub lane_count: u32,
    // pub speed_limit: f32, // m/s
}

impl Road {}

/// Helper function to spawn a new road entity with all necessary components
fn spawn_road(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &mut ResMut<RoadNetwork>,
    start_intersection_entity: IntersectionEntity,
    end_intersection_entity: IntersectionEntity,
    start_pos: Vec3,
    end_pos: Vec3,
) -> Result<RoadEntity> {
    const ROAD_WIDTH: f32 = 0.4;
    const ROAD_HEIGHT: f32 = 0.02;
    let road_color = Color::srgb(0.2, 0.2, 0.2);

    // Calculate angle from positions
    let direction = (end_pos - start_pos).normalize();
    let angle = direction.x.atan2(direction.z);

    // Calculate road properties
    let length = start_pos.distance(end_pos);
    let midpoint = (start_pos + end_pos) / 2.0;

    // Use the calculated angle for rotation
    let rotation = Quat::from_rotation_y(angle);

    let road = Road {
        start_intersection_entity,
        end_intersection_entity,
        length,
        angle,
        // lane_count: 2,     // Default 2 lanes
        // speed_limit: 13.4, // Default ~30 mph in m/s
    };

    // Spawn road segment
    let road_entity = commands
        .spawn((
            road,
            Mesh3d(meshes.add(Cuboid::new(ROAD_WIDTH, ROAD_HEIGHT, length))),
            MeshMaterial3d(materials.add(road_color)),
            Transform::from_translation(Vec3::new(midpoint.x, ROAD_HEIGHT / 2.0, midpoint.z))
                .with_rotation(rotation),
        ))
        .id();

    let road_entity_wrapper = RoadEntity(road_entity);

    // Add to network with the Road component
    road_network.add_road(
        road_entity_wrapper,
        &road,
    );

    Ok(road_entity_wrapper)
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
) -> Result<RoadEntity> {
    let start_intersection_entity = spawn_intersection(
        commands,
        meshes,
        materials,
        road_network,
        start_pos,
    )?;

    let end_intersection_entity = spawn_intersection(
        commands,
        meshes,
        materials,
        road_network,
        end_pos,
    )?;

    // Create and spawn road using positions directly
    let road_entity = spawn_road(
        commands,
        meshes,
        materials,
        road_network,
        start_intersection_entity,
        end_intersection_entity,
        start_pos,
        end_pos,
    )?;

    Ok(road_entity)
}

/// System to spawn roads connecting intersections and houses with driveways
pub fn spawn_roads(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut road_network: ResMut<RoadNetwork>,
) {
    // Define main road intersection positions - simplified loop structure
    let road_positions = vec![
        Vec3::new(0.0, 0.0, 20.0),    // 0 - Bottom main intersection
        Vec3::new(0.0, 0.0, -20.0),   // 1 - Top intersection
        Vec3::new(-10.0, 0.0, 0.0),   // 2 - Left middle intersection
        Vec3::new(10.0, 0.0, 0.0),    // 3 - Right middle intersection
    ];

    // First, create all road intersections
    let mut intersection_entities = Vec::new();

    for position in &road_positions {
        let intersection_entity = spawn_intersection(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut road_network,
            *position,
        ).expect("Failed to spawn intersection");

        intersection_entities.push(intersection_entity);
    }

    // Create road connections - a simple loop
    // Bottom -> Top (straight up)
    // Top -> Left middle -> Bottom (left path back)
    // Top -> Right middle -> Bottom (right path back)
    let road_connections = vec![
        (0, 1),  // Bottom to Top
        (1, 2),  // Top to Left middle
        (2, 0),  // Left middle to Bottom
        (1, 3),  // Top to Right middle
        (3, 0),  // Right middle to Bottom
    ];

    for (start_idx, end_idx) in road_connections {
        let start_pos = road_positions[start_idx];
        let end_pos = road_positions[end_idx];
        let start_intersection_entity = intersection_entities[start_idx];
        let end_intersection_entity = intersection_entities[end_idx];

        if let Err(e) = spawn_road(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut road_network,
            start_intersection_entity,
            end_intersection_entity,
            start_pos,
            end_pos,
        ) {
            bevy::log::error!("Failed to spawn road: {:#}", e);
        }
    }

    // Spawn 5 houses all connected to the bottom intersection
    let house_configs = vec![
        (Vec3::new(-8.0, 0.0, 25.0), 0),   // House 1 - South of bottom intersection
        (Vec3::new(-4.0, 0.0, 25.0), 0),   // House 2 - South of bottom intersection
        (Vec3::new(0.0, 0.0, 26.0), 0),    // House 3 - South of bottom intersection
        (Vec3::new(4.0, 0.0, 25.0), 0),    // House 4 - South of bottom intersection
        (Vec3::new(8.0, 0.0, 25.0), 0),    // House 5 - South of bottom intersection
        (Vec3::new(0.0, 0.0, -25.0), 1),    // House 6 - South of top intersection
        (Vec3::new(-4.0, 0.0, -25.0), 1),    // House 6 - South of top intersection
        (Vec3::new(4.0, 0.0, -25.0), 1),    // House 6 - South of top intersection
    ];

    for (house_pos, road_intersection_idx) in house_configs {
        let road_intersection_entity = intersection_entities[road_intersection_idx];
        let road_intersection_pos = road_positions[road_intersection_idx];

        if let Err(e) = crate::house::spawn_house_with_driveway(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut road_network,
            house_pos,
            road_intersection_entity,
            road_intersection_pos,
        ) {
            bevy::log::error!("Failed to spawn house with driveway: {:#}", e);
        }
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
