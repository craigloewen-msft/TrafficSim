use anyhow::{Context, Result};
use bevy::prelude::*;

use crate::car::Car;
use crate::intersection::{self, Intersection, IntersectionEntity, TrafficControlType};
use crate::road;
use crate::road_network::RoadNetwork;

// Import the spawn helper function
use crate::intersection::spawn_intersection;

/// Wrapper type for road entities to provide type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoadEntity(pub Entity);

/// Component that marks an entity as a road segment
#[derive(Component, Debug)]
pub struct Road {
    pub start_intersection_entity: IntersectionEntity,
    pub end_intersection_entity: IntersectionEntity,
    pub lane_count: u32,
    pub speed_limit: f32, // m/s
    pub angle: f32,       // Rotation angle in radians (Y-axis rotation)
}

impl Road {}

/// Helper function to spawn a new road entity with all necessary components
fn spawn_road(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    intersection_query: &Query<(&Intersection, &Transform), Without<Car>>,
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

    // Spawn road segment
    let road_entity = commands
        .spawn((
            Road {
                start_intersection_entity,
                end_intersection_entity,
                lane_count: 2,     // Default 2 lanes
                speed_limit: 13.4, // Default ~30 mph in m/s
                angle,
            },
            Mesh3d(meshes.add(Cuboid::new(ROAD_WIDTH, ROAD_HEIGHT, length))),
            MeshMaterial3d(materials.add(road_color)),
            Transform::from_translation(Vec3::new(midpoint.x, ROAD_HEIGHT / 2.0, midpoint.z))
                .with_rotation(rotation),
        ))
        .id();

    let road_entity_wrapper = RoadEntity(road_entity);

    // Add to network
    road_network.add_road(
        road_entity_wrapper,
        start_intersection_entity,
        end_intersection_entity,
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
    intersection_query: &Query<(&Intersection, &Transform), Without<Car>>,
    start_pos: Vec3,
    end_pos: Vec3,
) -> Result<RoadEntity> {
    let start_intersection_entity = spawn_intersection(
        commands,
        meshes,
        materials,
        start_pos,
        TrafficControlType::None,
    )?;

    let end_intersection_entity = spawn_intersection(
        commands,
        meshes,
        materials,
        end_pos,
        TrafficControlType::None,
    )?;

    // Create and spawn road using positions directly
    let road_entity = spawn_road(
        commands,
        meshes,
        materials,
        intersection_query,
        road_network,
        start_intersection_entity,
        end_intersection_entity,
        start_pos,
        end_pos,
    )?;

    Ok(road_entity)
}

/// System to spawn roads connecting houses
pub fn spawn_roads(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut road_network: ResMut<RoadNetwork>,
    intersection_query: Query<(&Intersection, &Transform), Without<Car>>,
) {
    // Define house positions (should match house.rs)
    let house_positions = vec![
        Vec3::new(-8.0, 0.0, -8.0),
        Vec3::new(8.0, 0.0, -8.0),
        Vec3::new(-8.0, 0.0, 8.0),
        Vec3::new(8.0, 0.0, 8.0),
        Vec3::new(-14.0, 0.0, -8.0),
    ];

    // First, create all intersections at house positions
    let mut intersection_entities = Vec::new();

    for position in &house_positions {
        let intersection_entity = spawn_intersection(
            &mut commands,
            &mut meshes,
            &mut materials,
            *position,
            TrafficControlType::StopSign,
        ).expect("Failed to spawn intersection");

        road_network.add_intersection(intersection_entity);

        intersection_entities.push(intersection_entity);
    }

    // Create road connections between intersections
    let road_connections = vec![
        (0, 1), // Left to Right (top)
        (2, 3), // Left to Right (bottom)
        (0, 2), // Top to Bottom (left)
        (1, 3), // Top to Bottom (right)
        (4, 0), // Extra house connection
    ];

    for (start_idx, end_idx) in road_connections {
        let start_pos = house_positions[start_idx];
        let end_pos = house_positions[end_idx];
        let start_intersection_entity = intersection_entities[start_idx];
        let end_intersection_entity = intersection_entities[end_idx];

        // Create and spawn road using positions directly (intersections not queryable yet)
        if let Err(e) = spawn_road(
            &mut commands,
            &mut meshes,
            &mut materials,
            &intersection_query,
            &mut road_network,
            start_intersection_entity,
            end_intersection_entity,
            start_pos,
            end_pos,
        ) {
            bevy::log::error!("Failed to spawn road: {:#}", e);
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
