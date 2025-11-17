use anyhow::Result;
use bevy::prelude::*;

use crate::intersection::IntersectionEntity;
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
    // pub lane_count: u32,
    // pub speed_limit: f32, // m/s
    pub angle: f32, // Rotation angle in radians (Y-axis rotation)
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

    // Spawn road segment
    let road_entity = commands
        .spawn((
            Road {
                start_intersection_entity,
                end_intersection_entity,
                // lane_count: 2,     // Default 2 lanes
                // speed_limit: 13.4, // Default ~30 mph in m/s
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
    start_pos: Vec3,
    end_pos: Vec3,
) -> Result<RoadEntity> {
    let start_intersection_entity =
        spawn_intersection(commands, meshes, materials, road_network, start_pos)?;

    let end_intersection_entity =
        spawn_intersection(commands, meshes, materials, road_network, end_pos)?;

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
    // Define main road intersection positions (these form the primary road network)
    let road_positions = vec![
        // Row 1 (top)
        Vec3::new(-20.0, 0.0, -20.0), // 0
        Vec3::new(-6.0, 0.0, -20.0),  // 1
        Vec3::new(8.0, 0.0, -20.0),   // 2
        Vec3::new(22.0, 0.0, -20.0),  // 3
        // Row 2 (middle)
        Vec3::new(-20.0, 0.0, 0.0), // 4
        Vec3::new(-6.0, 0.0, 0.0),  // 5
        Vec3::new(8.0, 0.0, 0.0),   // 6
        Vec3::new(22.0, 0.0, 0.0),  // 7
        // Row 3 (bottom)
        Vec3::new(-20.0, 0.0, 20.0), // 8
        Vec3::new(-6.0, 0.0, 20.0),  // 9
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
        )
        .expect("Failed to spawn intersection");

        intersection_entities.push(intersection_entity);
    }

    // Create road connections between intersections
    // Grid connections only - horizontal and vertical
    let road_connections = vec![
        // Row 1 horizontal
        (0, 1),
        (1, 2),
        (2, 3),
        // Row 2 horizontal
        (4, 5),
        (5, 6),
        (6, 7),
        // Row 3 horizontal
        (8, 9),
        // Column 1 vertical
        (0, 4),
        (4, 8),
        // Column 2 vertical
        (1, 5),
        (5, 9),
        // Column 3 vertical
        (2, 6),
        // Column 4 vertical
        (3, 7),
        // Some diagonal connections for interest
        (0, 5),
        (1, 6),
        (2, 7),
        (4, 9),
        (5, 6),
        (1, 4),
        (2, 5),
        (6, 9),
    ];

    for (start_idx, end_idx) in road_connections {
        let start_pos = road_positions[start_idx];
        let end_pos = road_positions[end_idx];
        let start_intersection_entity = intersection_entities[start_idx];
        let end_intersection_entity = intersection_entities[end_idx];

        // Create and spawn road using positions directly (intersections not queryable yet)
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

    // Now spawn houses with driveways connecting to nearby road intersections
    // Define house positions and which road intersection they connect to
    let house_configs = vec![
        // Houses along the top road (offset to the north)
        (Vec3::new(-20.0, 0.0, -25.0), 0), // House north of intersection 0
        (Vec3::new(-6.0, 0.0, -25.0), 1),  // House north of intersection 1
        (Vec3::new(8.0, 0.0, -25.0), 2),   // House north of intersection 2
        (Vec3::new(22.0, 0.0, -25.0), 3),  // House north of intersection 3
        // Houses along the left side (offset to the west)
        (Vec3::new(-25.0, 0.0, 0.0), 4), // House west of intersection 4
        (Vec3::new(-25.0, 0.0, 20.0), 8), // House west of intersection 8
        // Houses along the right side (offset to the east)
        (Vec3::new(27.0, 0.0, -20.0), 3), // House east of intersection 3
        (Vec3::new(27.0, 0.0, 0.0), 7),   // House east of intersection 7
        // Houses along the middle (offset to the south)
        (Vec3::new(-6.0, 0.0, 5.0), 5), // House south of intersection 5
        (Vec3::new(8.0, 0.0, 5.0), 6),  // House south of intersection 6
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
