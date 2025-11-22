use anyhow::Result;
use bevy::prelude::*;

use crate::road_network::RoadNetwork;
use crate::car::CarEntity;

/// Wrapper type to make it clear this Entity refers to an Intersection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntersectionEntity(pub Entity);

/// Component that marks an entity as an intersection
#[derive(Component, Debug)]
pub struct Intersection {
    /// The car currently occupying the intersection (if any)
    /// In FourWayStop mode, only one car can occupy at a time
    pub occupied_by: Option<CarEntity>,
    
    /// Timer for how long the current car has been in the intersection
    pub occupation_timer: f32,
    
    /// Time it takes for a car to cross through the intersection
    /// Other cars must wait until this time has elapsed
    pub crossing_time: f32,
}

impl Intersection {
    /// Create a new intersection with FourWayStop mode (default)
    pub fn new() -> Self {
        Self {
            occupied_by: None,
            occupation_timer: 0.0,
            crossing_time: 0.25, // Time it takes to cross the intersection (1 second)
        }
    }
    
    /// Release the intersection lock
    pub fn release(&mut self, car_entity: CarEntity) {
        if let Some(current_car) = self.occupied_by {
            if current_car == car_entity {
                self.occupied_by = None;
                self.occupation_timer = 0.0;
            }
        }
    }
    
    /// Check if a car can proceed through the intersection
    /// This handles both acquiring the lock and checking wait time
    /// Returns true if the car can proceed, false if it must wait
    pub fn can_proceed(&mut self, car_entity: CarEntity) -> bool {
        match self.occupied_by {
            None => {
                // Intersection is free, acquire it and start crossing
                self.occupied_by = Some(car_entity);
                self.occupation_timer = 0.0;
                false // Must wait the crossing time
            }
            Some(current_car) if current_car == car_entity => {
                // This car already has the lock, check if crossing time has elapsed
                self.occupation_timer >= self.crossing_time
            }
            Some(_) => {
                // Another car has the lock, must wait
                false
            }
        }
    }
    
    /// Update the occupation timer
    pub fn update_timer(&mut self, delta_time: f32) {
        if self.occupied_by.is_some() {
            self.occupation_timer += delta_time;
        }
    }
}

impl Default for Intersection {
    fn default() -> Self {
        Self::new()
    }
}

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
            Intersection::new(),
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

/// System to update intersection occupation timers
/// This prevents cars from blocking intersections indefinitely
pub fn update_intersections(
    time: Res<Time>,
    mut intersection_query: Query<&mut Intersection>,
) {
    for mut intersection in intersection_query.iter_mut() {
        intersection.update_timer(time.delta_secs());
    }
}

/// Plugin to register all intersection-related systems
pub struct IntersectionPlugin;

impl Plugin for IntersectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, update_intersections);
    }
}
