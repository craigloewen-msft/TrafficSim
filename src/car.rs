use crate::intersection::{Intersection, IntersectionEntity};
use crate::road::{Road, RoadEntity};
use crate::road_network::RoadNetwork;
use anyhow::{Context, Result};
use bevy::log::{error, info, warn};
use bevy::prelude::*;
use rand::seq::{IndexedRandom, IteratorRandom};

/// Component that marks an entity as a car
#[derive(Component)]
pub struct Car {
    pub speed: f32,
    pub max_speed: f32,
    pub current_road_entity: RoadEntity, // The road entity the car is currently on
    pub progress: f32,                   // 0.0 to 1.0 along the current road
    pub start_intersection: IntersectionEntity, // The intersection where we started on this road
    pub target_intersection: IntersectionEntity, // The intersection we're traveling toward
    pub final_target_intersection: IntersectionEntity, // The final destination intersection
    pub path: Vec<IntersectionEntity>, // Path of intersection entities to follow to reach the final destination
}

impl Car {
    /// Create a new car with default speed values
    pub fn new(
        current_road_entity: RoadEntity,
        start_intersection: IntersectionEntity,
        target_intersection: IntersectionEntity,
        final_target_intersection: IntersectionEntity,
        path: Vec<IntersectionEntity>,
    ) -> Self {
        Self {
            speed: 4.0,
            max_speed: 5.0,
            current_road_entity,
            progress: 0.0,
            start_intersection,
            target_intersection,
            final_target_intersection,
            path,
        }
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

/// Helper function to spawn a single car with proper error handling
fn try_spawn_car(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &RoadNetwork,
    road_query: &Query<(Entity, &Road)>,
    intersection_query: &Query<&Intersection>,
    road_entity: Entity,
    final_target_entity: Entity,
) -> Result<()> {
    let (_, road) = road_query
        .get(road_entity)
        .context("Failed to query road entity")?;

    let start_intersection = intersection_query
        .get(road.start_intersection_entity.0)
        .context("Failed to get start intersection")?;

    let spawn_pos = start_intersection.position + Vec3::new(0.0, 0.3, 0.0);
    info!("Spawning car at position: {:.2?}", spawn_pos);

    let start_entity = road.start_intersection_entity.0;

    let destination_intersection = intersection_query
        .get(final_target_entity)
        .context("Failed to query destination intersection")?;

    info!(
        "Car final destination: intersection at position {:.2?}",
        destination_intersection.position
    );

    let path = road_network
        .find_path(
            IntersectionEntity(start_entity),
            IntersectionEntity(final_target_entity),
        )
        .context("No path found from start to destination")?;

    let path_positions = path.iter().map(|intersection_entity| {
        intersection_query
            .get(intersection_entity.0)
            .map(|intersection| intersection.position)
            .unwrap_or(Vec3::ZERO)
    }).collect::<Vec<Vec3>>();

    commands.spawn(CarBundle {
        car: Car::new(
            RoadEntity(road_entity),
            road.start_intersection_entity,
            road.end_intersection_entity,
            IntersectionEntity(final_target_entity),
            path,
        ),
        mesh: Mesh3d(meshes.add(Cuboid::new(0.3, 0.2, 0.5))),
        material: MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))),
        transform: Transform::from_translation(spawn_pos)
            .with_rotation(Quat::from_rotation_y(road.angle)),
    });

    info!("✓ Car spawned successfully with path: {:?}!", path_positions);

    Ok(())
}

/// System to spawn cars in the world
pub fn spawn_cars(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    road_network: Res<RoadNetwork>,
    road_query: Query<(Entity, &Road)>,
    intersection_query: Query<&Intersection>,
) {
    info!("=== SPAWNING CARS ===");

    let num_cars_to_spawn = 1;
    let mut rng = rand::rng();

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

    for _ in 0..num_cars_to_spawn {
        // Choose a random road and destination
        let Some(&road_entity) = road_entities.choose(&mut rng) else {
            warn!("Failed to choose random road!");
            continue;
        };

        let Ok((_, road)) = road_query.get(road_entity) else {
            warn!("Failed to query road entity");
            continue;
        };

        let start_entity = road.start_intersection_entity.0;

        let Some(&final_target_entity) = all_intersections
            .iter()
            .filter(|&&entity| entity != start_entity)
            .choose(&mut rng)
        else {
            warn!("Could not find valid final destination!");
            continue;
        };

        // Try to spawn the car, log error if it fails
        if let Err(e) = try_spawn_car(
            &mut commands,
            &mut meshes,
            &mut materials,
            &road_network,
            &road_query,
            &intersection_query,
            road_entity,
            final_target_entity,
        ) {
            error!("Failed to spawn car: {:#}", e);
        }
    }
}

/// Helper function to update a single car with proper error handling
fn try_update_car(
    car_entity: Entity,
    car: &mut Car,
    transform: &mut Transform,
    delta_secs: f32,
    road_network: &RoadNetwork,
    road_query: &Query<(Entity, &Road)>,
    intersection_query: &Query<&Intersection>,
) -> Result<()> {
    // Check if we've reached the final destination
    if car.target_intersection.0 == car.final_target_intersection.0 && car.progress >= 1.0 {
        return Ok(()); // Car has stopped at destination
    }

    // Get the current road
    road_query
        .get(car.current_road_entity.0)
        .context("Road entity not found")?;

    // Get start and end intersection positions
    let start_intersection = intersection_query
        .get(car.start_intersection.0)
        .context("Start intersection not found")?;

    let target_intersection = intersection_query
        .get(car.target_intersection.0)
        .context("Target intersection not found")?;

    let start_pos = start_intersection.position;
    let end_pos = target_intersection.position;

    // Calculate road length
    let road_length = start_pos.distance(end_pos);
    if road_length < 0.01 {
        anyhow::bail!("Road length too short: {:.4}", road_length);
    }

    // Update progress along the road
    let progress_delta = (car.speed / road_length) * delta_secs;
    car.progress += progress_delta;

    // Check if we've reached the end of the current road
    if car.progress >= 1.0 {
        // We've reached the target intersection
        if car.target_intersection.0 == car.final_target_intersection.0 {
            info!("Car {:?} reached final destination", car_entity);
            car.progress = 1.0;
            transform.translation = end_pos;
            return Ok(());
        }

        // Get next intersection from path
        if car.path.is_empty() {
            anyhow::bail!("No path available to continue");
        }

        let next_intersection_entity = car.path.remove(0);

        // Find the road connecting to next intersection
        let next_road_entity = road_network
            .find_road_between(car.target_intersection, next_intersection_entity)
            .context("No road found between current and next intersection")?;

        car.current_road_entity = next_road_entity;
        car.progress = 0.0;

        // Determine direction on new road
        let (_, new_road) = road_query
            .get(next_road_entity.0)
            .context("Failed to query next road")?;

        if new_road.start_intersection_entity == car.target_intersection {
            // Travel from start to end
            car.start_intersection = new_road.start_intersection_entity;
            car.target_intersection = new_road.end_intersection_entity;
            transform.rotation = Quat::from_rotation_y(new_road.angle);
        } else if new_road.end_intersection_entity == car.target_intersection {
            // Travel from end to start
            car.start_intersection = new_road.end_intersection_entity;
            car.target_intersection = new_road.start_intersection_entity;
            transform.rotation = Quat::from_rotation_y(new_road.angle + std::f32::consts::PI);
        } else {
            anyhow::bail!("New road doesn't connect to current intersection");
        }

        let new_target_position = intersection_query
            .get(car.target_intersection.0)
            .context("Failed to get new target intersection")?
            .position;

        info!("Car moving to new position: {:.2?}", new_target_position);
    } else {
        // Interpolate position along current road
        transform.translation = start_pos.lerp(end_pos, car.progress);
    }

    Ok(())
}

/// System to update car movement logic
pub fn update_cars(
    time: Res<Time>,
    road_network: Res<RoadNetwork>,
    road_query: Query<(Entity, &Road)>,
    intersection_query: Query<&Intersection>,
    mut car_query: Query<(Entity, &mut Car, &mut Transform)>,
) {
    for (entity, mut car, mut transform) in car_query.iter_mut() {
        if let Err(e) = try_update_car(
            entity,
            &mut car,
            &mut transform,
            time.delta_secs(),
            &road_network,
            &road_query,
            &intersection_query,
        ) {
            error!("Car {:?} update failed: {:#}", entity, e);
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
