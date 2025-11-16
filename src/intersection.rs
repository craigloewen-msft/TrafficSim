use anyhow::Result;
use bevy::prelude::*;

use crate::road_network::RoadNetwork;

/// Wrapper type to make it clear this Entity refers to an Intersection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntersectionEntity(pub Entity);

/// Component that marks an entity as an intersection
#[derive(Component, Debug)]
pub struct Intersection {}

impl Intersection {}

/// Helper function to spawn an intersection entity with visual representation
/// This is the ONE function to spawn intersections - it automatically adds to the road network
pub fn spawn_intersection(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &mut ResMut<RoadNetwork>,
    position: Vec3,
) -> Result<IntersectionEntity> {
    const INTERSECTION_SIZE: f32 = 0.6;
    const INTERSECTION_HEIGHT: f32 = 0.03;
    let intersection_color = Color::srgb(0.3, 0.3, 0.3);

    let entity = commands
        .spawn((
            Intersection {},
            Mesh3d(meshes.add(Cuboid::new(
                INTERSECTION_SIZE,
                INTERSECTION_HEIGHT,
                INTERSECTION_SIZE,
            ))),
            MeshMaterial3d(materials.add(intersection_color)),
            Transform::from_translation(Vec3::new(
                position.x,
                INTERSECTION_HEIGHT / 2.0,
                position.z,
            )),
        ))
        .id();

    let intersection_entity = IntersectionEntity(entity);
    
    // Always add to road network
    road_network.add_intersection(intersection_entity);

    Ok(intersection_entity)
}
