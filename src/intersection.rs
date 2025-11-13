use bevy::prelude::*;

/// Unique identifier for intersections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntersectionId(pub u32);

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
    pub id: IntersectionId,
    pub position: Vec3,
    pub connected_roads: Vec<Entity>,
    pub traffic_control: TrafficControlType,
}

impl Intersection {
    /// Creates a new intersection at the given position
    pub fn new(id: IntersectionId, position: Vec3, traffic_control: TrafficControlType) -> Self {
        Self {
            id,
            position,
            connected_roads: Vec::new(),
            traffic_control,
        }
    }

    /// Spawns the intersection entity with visual representation
    pub fn spawn(
        &self,
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<StandardMaterial>>,
    ) -> Entity {
        const INTERSECTION_SIZE: f32 = 0.6;
        const INTERSECTION_HEIGHT: f32 = 0.03;
        let intersection_color = Color::srgb(0.3, 0.3, 0.3);

        commands.spawn((
            Intersection {
                id: self.id,
                position: self.position,
                connected_roads: Vec::new(),
                traffic_control: self.traffic_control,
            },
            Mesh3d(meshes.add(Cuboid::new(INTERSECTION_SIZE, INTERSECTION_HEIGHT, INTERSECTION_SIZE))),
            MeshMaterial3d(materials.add(intersection_color)),
            Transform::from_translation(Vec3::new(
                self.position.x,
                INTERSECTION_HEIGHT / 2.0,
                self.position.z,
            )),
        )).id()
    }
}

/// Data stored in the road network for pathfinding
#[derive(Debug, Clone)]
pub struct IntersectionData {
    pub position: Vec3,
    pub entity: Entity,
    pub traffic_control: TrafficControlType,
}
