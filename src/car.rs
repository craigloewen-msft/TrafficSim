use crate::intersection::{Intersection, IntersectionEntity};
use crate::road::{Road, RoadEntity};
use crate::road_network::RoadNetwork;
use anyhow::{Context, Result};
use bevy::log::{error, info, warn};
use bevy::prelude::*;
use ordered_float::OrderedFloat;
use rand::seq::IndexedRandom;
use rand::Rng;

/// Wrapper type for car entities to provide type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CarEntity(pub Entity);

/// Component that marks an entity as a car
#[derive(Component, Clone)]
pub struct Car {
    pub speed: f32,                             // Speed of the car
    pub current_road_entity: RoadEntity,        // The road entity the car is currently on
    pub progress: OrderedFloat<f32>,            // 0.0 to 1.0 along the current road
    pub start_intersection: IntersectionEntity, // The intersection where we started on this road
    pub path: Vec<IntersectionEntity>, // Path of intersection entities to follow (first element is next target)
}

impl Car {
    /// Update car movement logic with proper error handling
    /// Returns `true` if the car should be despawned (reached destination or error)
    pub fn update_car(
        &mut self,
        car_entity: &CarEntity,
        transform: &mut Transform,
        delta_secs: f32,
        road_network: &mut RoadNetwork,
        road_query: &Query<(Entity, &Road)>,
        intersection_query: &Query<(&Intersection, &Transform), Without<Car>>,
    ) -> Result<bool> {
        // Check if we've reached the final destination
        if self.path.is_empty() {
            return Ok(true); // Signal that car should be despawned
        }

        // Get the current road
        let current_road_queried = road_query
            .get(self.current_road_entity.0)
            .context("Road entity not found")?;

        // Get the target intersection (first item in path)
        let target_intersection_entity = self
            .path
            .first()
            .context("Path is empty, no target intersection")?;

        // Get start and end intersection positions
        let start_intersection_transform = intersection_query
            .get(self.start_intersection.0)
            .context("Start intersection not found")?;

        let target_intersection_transform = intersection_query
            .get(target_intersection_entity.0)
            .context("Target intersection not found")?;

        let start_pos = start_intersection_transform.1.translation;
        let end_pos = target_intersection_transform.1.translation;

        // Calculate road length
        let road_length = current_road_queried.1.length;

        let prev_road_entity = self.current_road_entity;
        let prev_progress = self.progress;

        let ahead_car_option = road_network
            .find_car_ahead_on_road(self.current_road_entity, &self.progress)
            .context("Error on ahead car")?;

        // Update progress along the road
        let mut progress_delta = (self.speed / road_length) * delta_secs;

        if let Some(ahead_car) = ahead_car_option {
            let ahead_car_progress_diff = ahead_car.0 - self.progress;
            if ahead_car_progress_diff <= OrderedFloat(progress_delta) {
                progress_delta = 0.0;
            }
        }

        self.progress += progress_delta;

        // Check if we've reached the end of the current road
        if self.progress >= OrderedFloat(1.0) {
            // Remove the intersection we just reached from the path
            let reached_intersection = self.path.remove(0);

            if self.path.is_empty() {
                info!("Car {:?} reached final destination", car_entity);
                self.progress = OrderedFloat(1.0);
                transform.translation = end_pos;

                road_network
                    .update_car_road_position(
                        &self,
                        car_entity,
                        true,
                        Some(prev_road_entity),
                        prev_progress,
                    )
                    .context("Failed to update car position")?;

                return Ok(true); // Signal that car should be despawned
            }

            let next_intersection_entity =
                *self.path.first().context("No next intersection in path")?;

            // Find the road connecting to next intersection
            let next_road_entity = road_network
                .find_road_between(reached_intersection, next_intersection_entity)
                .context("No road found between current and next intersection")?;

            self.current_road_entity = next_road_entity;
            self.progress = OrderedFloat(0.0);

            let (_, new_road) = road_query
                .get(next_road_entity.0)
                .context("Failed to query next road")?;

            self.start_intersection = new_road.start_intersection_entity;
            transform.rotation = Quat::from_rotation_y(new_road.angle);

            let new_target_position = intersection_query
                .get(next_intersection_entity.0)
                .context("Failed to get new target intersection")?
                .1
                .translation;

            info!(
                "Car {:?} moving to new position: {:.2?}",
                car_entity, new_target_position
            );
        } else {
            // Interpolate position along current road
            transform.translation = start_pos.lerp(end_pos, self.progress.into_inner());
        }

        road_network
            .update_car_road_position(
                &self,
                car_entity,
                false,
                Some(prev_road_entity),
                prev_progress,
            )
            .context("Failed to update car position")?;

        Ok(false) // Car continues to exist
    }
}

/// Bundle for spawning a car with all necessary components
#[derive(Bundle)]
pub struct CarBundle {
    pub car: Car,
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
}

/// Helper function to spawn a single car
/// This is called by the system and can take any system parameters it needs
pub fn spawn_car(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &mut RoadNetwork,
    road_query: &Query<(Entity, &Road)>,
    intersection_query: &Query<(&Intersection, &Transform), Without<Car>>,
    spawn_intersection_entity: IntersectionEntity,
    road_entity: Entity,
    final_target_entity: Entity,
) -> Result<CarEntity> {
    let (_, road) = road_query
        .get(road_entity)
        .context("Failed to query road entity")?;

    if road.start_intersection_entity != spawn_intersection_entity {
        anyhow::bail!(
            "Road {:?} does not start from intersection {:?} (one-way roads)",
            road_entity,
            spawn_intersection_entity
        );
    }

    let spawn_intersection_transform = intersection_query
        .get(spawn_intersection_entity.0)
        .context("Failed to get spawn intersection")?;

    let spawn_pos = spawn_intersection_transform.1.translation + Vec3::new(0.0, 0.3, 0.0);

    let path = road_network
        .find_path(
            spawn_intersection_entity,
            IntersectionEntity(final_target_entity),
        )
        .context("No path found from start to destination")?;

    let path_positions: Vec<Vec3> = path
        .iter()
        .map(|intersection_entity| {
            intersection_query
                .get(intersection_entity.0)
                .context("Failed to find intersection for path pos")
                .map(|(_, transform)| transform.translation)
        })
        .collect::<Result<Vec<_>>>()?;

    let rotation = Quat::from_rotation_y(road.angle);

    // Generate random speed for variety - some cars faster, some slower
    let mut rng = rand::rng();
    let random_speed = rng.random_range(2.0..6.0); // Speed range from 2.0 to 6.0

    let car = Car {
        speed: random_speed,
        current_road_entity: RoadEntity(road_entity),
        progress: OrderedFloat(0.0),
        start_intersection: spawn_intersection_entity,
        path,
    };

    let car_clone = car.clone();

    // Spawn the entity with all components
    let entity = commands
        .spawn(CarBundle {
            car: car,
            mesh: Mesh3d(meshes.add(Cuboid::new(0.3, 0.2, 0.5))),
            material: MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))),
            transform: Transform::from_translation(spawn_pos).with_rotation(rotation),
        })
        .id();

    let car_entity = CarEntity(entity);

    road_network
        .update_car_road_position(&car_clone, &car_entity, false, None, OrderedFloat(0.0))
        .context("Failed to update road network")?;

    info!(
        "✓ Car {:?} spawning at {:.2?} and positions: {:?}",
        entity, spawn_pos, path_positions
    );

    Ok(car_entity)
}

/// System to spawn cars in the world
pub fn spawn_cars(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut road_network: ResMut<RoadNetwork>,
    road_query: Query<(Entity, &Road)>,
    intersection_query: Query<(&Intersection, &Transform), Without<Car>>,
) {
    info!("=== SPAWNING CARS ===");

    let num_cars_to_spawn = 0;

    // Collect road entities from the road query
    let road_entities: Vec<Entity> = road_query.iter().map(|(entity, _)| entity).collect();
    if road_entities.is_empty() {
        warn!("No roads available in road network!");
        return;
    }

    // Collect all intersection entities from the road network
    let all_intersections: Vec<Entity> = road_network
        .get_all_intersections()
        .iter()
        .map(|intersection_entity| intersection_entity.0)
        .collect();

    if all_intersections.len() < 2 {
        warn!("Not enough intersections for pathfinding (need at least 2)!");
        return;
    }

    let mut rng = rand::rng();

    for _ in 0..num_cars_to_spawn {
        // Choose a road and intersections
        let road_entity = *road_entities.choose(&mut rng).unwrap();
        let (_, road) = road_query.get(road_entity).unwrap();
        let spawn_intersection_entity = road.start_intersection_entity;
        let final_target_entity = *all_intersections.choose(&mut rng).unwrap();

        // Spawn the car entity - all ECS operations stay in the system
        if let Err(e) = spawn_car(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut road_network,
            &road_query,
            &intersection_query,
            spawn_intersection_entity,
            road_entity,
            final_target_entity,
        ) {
            error!("Failed to spawn car: {:#}", e);
        }
    }
}

/// System to update car movement logic
pub fn update_cars(
    time: Res<Time>,
    mut road_network: ResMut<RoadNetwork>,
    road_query: Query<(Entity, &Road)>,
    intersection_query: Query<(&Intersection, &Transform), Without<Car>>,
    mut car_query: Query<(Entity, &mut Car, &mut Transform)>,
    mut commands: Commands,
) {
    for (entity, mut car, mut transform) in car_query.iter_mut() {
        match car.update_car(
            &CarEntity(entity),
            &mut transform,
            time.delta_secs(),
            &mut road_network,
            &road_query,
            &intersection_query,
        ) {
            Ok(should_despawn) => {
                if should_despawn {
                    commands.entity(entity).despawn();
                }
            }
            Err(e) => {
                error!("Car {:?} update failed: {:#}", entity, e);
                commands.entity(entity).despawn();
            }
        }
    }
}

/// Plugin to register all car-related systems
pub struct CarPlugin;

impl Plugin for CarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_cars.after(crate::road::spawn_roads))
            .add_systems(Update, update_cars);
    }
}
