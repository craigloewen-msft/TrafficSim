use anyhow::Result;
use bevy::prelude::*;

use crate::intersection::{IntersectionEntity};
use crate::road_network::RoadNetwork;

// Import the spawn helper function
use crate::intersection::spawn_intersection;
use crate::two_way_road::spawn_two_way_road_between_intersections;

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
    pub is_two_way: bool, // Whether this is part of a two-way road (for visual lane offset)
    // pub lane_count: u32,
    // pub speed_limit: f32, // m/s
}

impl Road {}

/// Helper function to create a road logically (component + network registration)
/// Returns the RoadEntity wrapper and the Road component
pub fn add_road_logic(
    commands: &mut Commands,
    road_network: &mut ResMut<RoadNetwork>,
    start_intersection_entity: IntersectionEntity,
    end_intersection_entity: IntersectionEntity,
    start_pos: Vec3,
    end_pos: Vec3,
    is_two_way: bool,
) -> Result<RoadEntity> {
    // Calculate angle from positions
    let direction = (end_pos - start_pos).normalize();
    let angle = direction.x.atan2(direction.z);

    // Calculate road properties
    let length = start_pos.distance(end_pos);

    let road = Road {
        start_intersection_entity,
        end_intersection_entity,
        length,
        angle,
        is_two_way,
    };

    let road_entity = commands.spawn(road).id();
    let road_entity_wrapper = RoadEntity(road_entity);

    // Add to network with the Road component
    road_network.add_road(road_entity_wrapper, &road);

    Ok(road_entity_wrapper)
}

/// Helper function to spawn V-shaped directional arrow indicators on a road
/// Creates clear directional arrows using two cuboids in a V formation
/// 
/// # Arguments
/// * `offset_x` - Lateral offset from center (0.0 for centered, use ±0.15 for two-way roads)
pub fn spawn_direction_arrows(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    start_pos: Vec3,
    end_pos: Vec3,
    offset_x: f32,
    parent_entity: Entity,
) {
    const ARROW_ARM_WIDTH: f32 = 0.04;
    const ARROW_ARM_HEIGHT: f32 = 0.03;
    const ARROW_ARM_LENGTH: f32 = 0.15;
    const ARROW_ANGLE: f32 = 0.5;  // Angle in radians for the V shape
    const ARROW_SPACING: f32 = 2.0;
    let arrow_color = Color::srgb(0.9, 0.9, 0.3);

    let length = start_pos.distance(end_pos);

    // Calculate number of arrows based on road length
    let num_arrows = (length / ARROW_SPACING).max(1.0) as i32;

    for i in 0..num_arrows {
        let t = (i as f32 + 0.5) / num_arrows as f32;
        // Calculate position along the road's length (local Z-axis)
        let z_offset = (t - 0.5) * length;

        // Spawn V-shaped arrow as children of road - uses local space
        commands.entity(parent_entity).with_children(|parent| {
            // Left arm of the V (rotated counterclockwise)
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(ARROW_ARM_WIDTH, ARROW_ARM_HEIGHT, ARROW_ARM_LENGTH))),
                MeshMaterial3d(materials.add(arrow_color)),
                Transform::from_translation(Vec3::new(
                    offset_x - ARROW_ARM_LENGTH * 0.5 * ARROW_ANGLE.sin(),
                    ARROW_ARM_HEIGHT,
                    z_offset + ARROW_ARM_LENGTH * 0.5 * ARROW_ANGLE.cos(),
                ))
                .with_rotation(Quat::from_rotation_y(-ARROW_ANGLE)),
            ));

            // Right arm of the V (rotated clockwise)
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(ARROW_ARM_WIDTH, ARROW_ARM_HEIGHT, ARROW_ARM_LENGTH))),
                MeshMaterial3d(materials.add(arrow_color)),
                Transform::from_translation(Vec3::new(
                    offset_x + ARROW_ARM_LENGTH * 0.5 * ARROW_ANGLE.sin(),
                    ARROW_ARM_HEIGHT,
                    z_offset + ARROW_ARM_LENGTH * 0.5 * ARROW_ANGLE.cos(),
                ))
                .with_rotation(Quat::from_rotation_y(ARROW_ANGLE)),
            ));
        });
    }
}

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

    // Calculate properties for visual rendering
    let length = start_pos.distance(end_pos);
    let midpoint = (start_pos + end_pos) / 2.0;
    let direction = (end_pos - start_pos).normalize();
    let angle = direction.x.atan2(direction.z);
    let rotation = Quat::from_rotation_y(angle);

    // Create the road logically
    let road_entity_wrapper = add_road_logic(
        commands,
        road_network,
        start_intersection_entity,
        end_intersection_entity,
        start_pos,
        end_pos,
        false, // One-way road
    )?;

    // Add visual components to the existing entity
    commands.entity(road_entity_wrapper.0).insert((
        Mesh3d(meshes.add(Cuboid::new(ROAD_WIDTH, ROAD_HEIGHT, length))),
        MeshMaterial3d(materials.add(road_color)),
        Transform::from_translation(Vec3::new(midpoint.x, ROAD_HEIGHT / 2.0, midpoint.z))
            .with_rotation(rotation),
    ));

    // Add direction arrows (centered for one-way roads)
    spawn_direction_arrows(
        commands,
        meshes,
        materials,
        start_pos,
        end_pos,
        0.0,  // Centered offset
        road_entity_wrapper.0,
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
    ];

    for (start_idx, end_idx) in road_connections {
        let start_pos = road_positions[start_idx];
        let end_pos = road_positions[end_idx];
        let start_intersection_entity = intersection_entities[start_idx];
        let end_intersection_entity = intersection_entities[end_idx];

        if let Err(e) = spawn_two_way_road_between_intersections(
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
