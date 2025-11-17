use crate::intersection::{Intersection, IntersectionEntity};
use crate::road::{Road, RoadEntity};
use crate::road_network::RoadNetwork;
use anyhow::{Context, Result};
use bevy::log::{error, info, warn};
use bevy::prelude::*;
use rand::seq::IndexedRandom;

/// Wrapper type for car entities to provide type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CarEntity(pub Entity);

/// Component that marks an entity as a car
#[derive(Component)]
pub struct Car {
    pub speed: f32,
    pub current_road_entity: RoadEntity, // The road entity the car is currently on
    pub progress: f32,                   // 0.0 to 1.0 along the current road
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
        road_network: &RoadNetwork,
        road_query: &Query<(Entity, &Road)>,
        intersection_query: &Query<(&Intersection, &Transform), Without<Car>>,
    ) -> Result<bool> {
        // Check if we've reached the final destination
        if self.path.is_empty() {
            return Ok(true); // Signal that car should be despawned
        }

        // Get the current road
        road_query
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
        let road_length = start_pos.distance(end_pos);
        if road_length < 0.01 {
            anyhow::bail!("Road length too short: {:.4}", road_length);
        }

        // Update progress along the road
        let progress_delta = (self.speed / road_length) * delta_secs;
        self.progress += progress_delta;

        // Check if we've reached the end of the current road
        if self.progress >= 1.0 {
            // Remove the intersection we just reached from the path
            let reached_intersection = self.path.remove(0);

            if self.path.is_empty() {
                info!("Car {:?} reached final destination", car_entity);
                self.progress = 1.0;
                transform.translation = end_pos;

                return Ok(true); // Signal that car should be despawned
            } // Get next intersection from path (peek at the next target)
            let next_intersection_entity =
                *self.path.first().context("No next intersection in path")?;

            // Find the road connecting to next intersection
            let next_road_entity = road_network
                .find_road_between(reached_intersection, next_intersection_entity)
                .context("No road found between current and next intersection")?;

            self.current_road_entity = next_road_entity;
            self.progress = 0.0;

            // Determine direction on new road
            let (_, new_road) = road_query
                .get(next_road_entity.0)
                .context("Failed to query next road")?;

            if new_road.start_intersection_entity == reached_intersection {
                // Travel from start to end
                self.start_intersection = new_road.start_intersection_entity;
                transform.rotation = Quat::from_rotation_y(new_road.angle);
            } else if new_road.end_intersection_entity == reached_intersection {
                // Travel from end to start
                self.start_intersection = new_road.end_intersection_entity;
                transform.rotation = Quat::from_rotation_y(new_road.angle + std::f32::consts::PI);
            } else {
                anyhow::bail!("New road doesn't connect to current intersection");
            }

            let new_target_position = intersection_query
                .get(next_intersection_entity.0)
                .context("Failed to get new target intersection")?
                .1
                .translation;

            info!("Car {:?} moving to new position: {:.2?}", car_entity, new_target_position);
        } else {
            // Interpolate position along current road
            transform.translation = start_pos.lerp(end_pos, self.progress);
        }

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
    road_network: &RoadNetwork,
    road_query: &Query<(Entity, &Road)>,
    intersection_query: &Query<(&Intersection, &Transform), Without<Car>>,
    spawn_intersection_entity: IntersectionEntity,
    road_entity: Entity,
    final_target_entity: Entity,
) -> Result<CarEntity> {
    let (_, road) = road_query
        .get(road_entity)
        .context("Failed to query road entity")?;

    // Validate that the road is connected to the spawn intersection
    if road.start_intersection_entity != spawn_intersection_entity
        && road.end_intersection_entity != spawn_intersection_entity
    {
        anyhow::bail!(
            "Road {:?} is not connected to intersection {:?}",
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

    // Determine rotation based on which end of the road we're starting from
    let rotation = if road.start_intersection_entity == spawn_intersection_entity {
        Quat::from_rotation_y(road.angle)
    } else {
        Quat::from_rotation_y(road.angle + std::f32::consts::PI)
    };

    // Spawn the entity with all components
    let entity = commands
        .spawn(CarBundle {
            car: Car {
                speed: 4.0,
                current_road_entity: RoadEntity(road_entity),
                progress: 0.0,
                start_intersection: spawn_intersection_entity,
                path,
            },
            mesh: Mesh3d(meshes.add(Cuboid::new(0.3, 0.2, 0.5))),
            material: MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))),
            transform: Transform::from_translation(spawn_pos).with_rotation(rotation),
        })
        .id();

    info!(
        "✓ Car {:?} spawning at {:.2?} and positions: {:?}",
        entity, spawn_pos, path_positions
    );

    Ok(CarEntity(entity))
}

/// System to spawn cars in the world
pub fn spawn_cars(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    road_network: Res<RoadNetwork>,
    road_query: Query<(Entity, &Road)>,
    intersection_query: Query<(&Intersection, &Transform), Without<Car>>,
    mut stats: ResMut<crate::SimulationStats>,
) {
    info!("=== SPAWNING CARS ===");

    let num_cars_to_spawn = 5;

    // Collect road entities from the road query
    let road_entities: Vec<Entity> = road_query.iter().map(|(entity, _)| entity).collect();
    if road_entities.is_empty() {
        warn!("No roads available in road network!");
        return;
    }

    // Collect all intersection entities from the adjacency graph
    let all_intersections: Vec<Entity> = road_network
        .adjacency
        .keys()
        .map(|intersection_entity| intersection_entity.0)
        .collect();

    if all_intersections.len() < 2 {
        warn!("Not enough intersections for pathfinding (need at least 2)!");
        return;
    }

    let mut rng = rand::rng();
    let mut spawned_count = 0;

    for _ in 0..num_cars_to_spawn {
        // Choose a road and intersections
        let road_entity = *road_entities.choose(&mut rng).unwrap();
        let (_, road) = road_query.get(road_entity).unwrap();
        let spawn_intersection_entity = road.start_intersection_entity;
        let final_target_entity = *all_intersections.choose(&mut rng).unwrap();

        // Spawn the car entity - all ECS operations stay in the system
        if let Ok(_) = spawn_car(
            &mut commands,
            &mut meshes,
            &mut materials,
            &road_network,
            &road_query,
            &intersection_query,
            spawn_intersection_entity,
            road_entity,
            final_target_entity,
        ) {
            spawned_count += 1;
        } else {
            error!("Failed to spawn car");
        }
    }
    
    stats.total_cars_spawned += spawned_count;
    info!("Successfully spawned {} cars", spawned_count);
}

/// System to update car movement logic
pub fn update_cars(
    time: Res<Time>,
    road_network: Res<RoadNetwork>,
    road_query: Query<(Entity, &Road)>,
    intersection_query: Query<(&Intersection, &Transform), Without<Car>>,
    mut car_query: Query<(Entity, &mut Car, &mut Transform)>,
    mut commands: Commands,
    mut stats: ResMut<crate::SimulationStats>,
) {
    for (entity, mut car, mut transform) in car_query.iter_mut() {
        match car.update_car(
            &CarEntity(entity),
            &mut transform,
            time.delta_secs(),
            &road_network,
            &road_query,
            &intersection_query,
        ) {
            Ok(should_despawn) => {
                if should_despawn {
                    stats.total_cars_completed += 1;
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
