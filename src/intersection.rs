use bevy::prelude::*;
use anyhow::Result;

/// Wrapper type to make it clear this Entity refers to an Intersection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntersectionEntity(pub Entity);

/// Types of traffic control at intersections
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficControlType {
    None,           // No control, cars must yield
    StopSign,       // All-way stop
    TrafficLight,   // Controlled by signals
}

/// Component that marks an entity as an intersection
#[derive(Component, Debug)]
pub struct Intersection {
    pub traffic_control: TrafficControlType,
}

impl Intersection {
    /// Creates a new intersection with the given traffic control type
    pub fn new(traffic_control: TrafficControlType) -> Self {
        Self {
            traffic_control,
        }
    }
}

/// Helper function to spawn an intersection entity with visual representation
pub fn spawn_intersection(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    traffic_control: TrafficControlType,
) -> Result<IntersectionEntity> {
    const INTERSECTION_SIZE: f32 = 0.6;
    const INTERSECTION_HEIGHT: f32 = 0.03;
    let intersection_color = Color::srgb(0.3, 0.3, 0.3);

    let entity = commands.spawn((
        Intersection {
            traffic_control,
        },
        Mesh3d(meshes.add(Cuboid::new(INTERSECTION_SIZE, INTERSECTION_HEIGHT, INTERSECTION_SIZE))),
        MeshMaterial3d(materials.add(intersection_color)),
        Transform::from_translation(Vec3::new(
            position.x,
            INTERSECTION_HEIGHT / 2.0,
            position.z,
        )),
    )).id();

    Ok(IntersectionEntity(entity))
}
