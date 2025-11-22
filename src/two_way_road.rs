use anyhow::Result;
use bevy::prelude::*;

use crate::intersection::{IntersectionEntity};
use crate::road::{add_road_logic, spawn_direction_arrows};
use crate::road_network::RoadNetwork;

/// Wrapper type for two-way road entities to provide type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TwoWayRoadEntity(pub Entity);

/// Component that marks an entity as a two-way road (contains two logical roads)
#[derive(Component, Debug, Clone, Copy)]
pub struct TwoWayRoad {
}

/// Helper function to spawn a two-way road between existing intersections
/// This is the single source of truth for two-way road creation
/// Creates ONE visual mesh (wider) but TWO logical roads for pathfinding
pub fn spawn_two_way_road_between_intersections(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &mut ResMut<RoadNetwork>,
    start_intersection: IntersectionEntity,
    end_intersection: IntersectionEntity,
    start_pos: Vec3,
    end_pos: Vec3,
) -> Result<TwoWayRoadEntity> {
    const TWO_WAY_ROAD_WIDTH: f32 = 0.6;  // Wider than one-way road (0.4)
    const ROAD_HEIGHT: f32 = 0.02;
    let road_color = Color::srgb(0.2, 0.2, 0.2);

    // Calculate road properties
    let direction = (end_pos - start_pos).normalize();
    let angle = direction.x.atan2(direction.z);
    let length = start_pos.distance(end_pos);
    let midpoint = (start_pos + end_pos) / 2.0;
    let rotation = Quat::from_rotation_y(angle);

    // Create forward road (start to end) - logical only, no visual
    let _forward_road_wrapper = add_road_logic(
        commands,
        road_network,
        start_intersection,
        end_intersection,
        start_pos,
        end_pos,
        true, // Two-way road
    )?;

    // Create backward road (end to start) - logical only, no visual
    let _backward_road_wrapper = add_road_logic(
        commands,
        road_network,
        end_intersection,
        start_intersection,
        end_pos,
        start_pos,
        true, // Two-way road
    )?;

    // Create the visual two-way road entity (single wider mesh)
    let two_way_road = TwoWayRoad {
    };

    let two_way_road_entity = commands
        .spawn((
            two_way_road,
            Mesh3d(meshes.add(Cuboid::new(TWO_WAY_ROAD_WIDTH, ROAD_HEIGHT, length))),
            MeshMaterial3d(materials.add(road_color)),
            Transform::from_translation(Vec3::new(midpoint.x, ROAD_HEIGHT / 2.0, midpoint.z))
                .with_rotation(rotation),
        ))
        .id();

    // Add direction arrows for forward direction (left lane)
    spawn_direction_arrows(
        commands,
        meshes,
        materials,
        start_pos,
        end_pos,
        -0.15,  // Left lane offset
        two_way_road_entity,
    );

    // Add direction arrows for backward direction (right lane)
    spawn_direction_arrows(
        commands,
        meshes,
        materials,
        end_pos,
        start_pos,
        0.15,  // Right lane offset
        two_way_road_entity,
    );

    Ok(TwoWayRoadEntity(two_way_road_entity))
}
