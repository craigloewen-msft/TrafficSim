use crate::intersection::Intersection;
use crate::road::Road;
use crate::road_network::RoadNetwork;
use bevy::log::{info, warn};
use bevy::prelude::*;
use rand::Rng;

/// Component that marks an entity as a car
#[derive(Component)]
pub struct Car {
    pub speed: f32,
    pub max_speed: f32,
    pub current_road_entity: Option<Entity>, // The road entity the car is currently on
    pub progress: f32,                       // 0.0 to 1.0 along the current road
}

impl Default for Car {
    fn default() -> Self {
        Self {
            speed: 2.0,
            max_speed: 5.0,
            current_road_entity: None,
            progress: 0.0,
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

/// System to spawn cars in the world
pub fn spawn_cars(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    road_network: Res<RoadNetwork>,
    road_query: Query<&Road>,
    intersection_query: Query<&Intersection>,
) {
    info!("=== SPAWNING CARS ===");
    info!(
        "Road network contains {} roads",
        road_network.road_entities.len()
    );

    let num_cars_to_spawn = 5;
    let mut rng = rand::rng();
    
    // Collect road entities once before the loop
    let road_entities: Vec<_> = road_network.road_entities.values().collect();
    if road_entities.is_empty() {
        warn!("No roads available in road network!");
        return;
    }
    
    for _ in 0..num_cars_to_spawn {
        // Spawn a car on a random road
        let random_index = rng.random_range(0..road_entities.len());
        let &road_entity = road_entities[random_index];
        info!("Selected road entity: {:?}", road_entity);

        if let Ok(road) = road_query.get(road_entity) {
            info!(
                "Road details - start: {:?}, end: {:?}, angle: {:.2}",
                road.start_intersection, road.end_intersection, road.angle
            );

            // Get the start intersection position
            if let Ok(start_intersection) = intersection_query.get(road.start_intersection) {
                let spawn_pos = start_intersection.position + Vec3::new(0.0, 0.15, 0.0);
                info!("Spawning car at position: {:.2?}", spawn_pos);

                commands.spawn(CarBundle {
                    car: Car {
                        current_road_entity: Some(road_entity),
                        progress: 0.0,
                        ..default()
                    },
                    mesh: Mesh3d(meshes.add(Cuboid::new(0.3, 0.2, 0.5))),
                    material: MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))),
                    transform: Transform::from_translation(spawn_pos)
                        .with_rotation(Quat::from_rotation_y(road.angle)),
                });

                info!("✓ Car spawned successfully!");
            } else {
                warn!(
                    "Failed to get start intersection {:?}",
                    road.start_intersection
                );
            }
        } else {
            warn!("Failed to query road entity {:?}", road_entity);
        }
    }
}

/// System to update car movement logic
pub fn update_cars(
    time: Res<Time>,
    road_network: Res<RoadNetwork>,
    road_query: Query<&Road>,
    intersection_query: Query<&Intersection>,
    mut car_query: Query<(Entity, &mut Car, &mut Transform)>,
) {
    for (entity, mut car, mut transform) in car_query.iter_mut() {
        // debug!("Car {:?}: progress={:.3}, speed={:.2}, pos={:.2?}",
        //        entity, car.progress, car.speed, transform.translation);

        // Get the current road the car is on
        let Some(current_road_entity) = car.current_road_entity else {
            warn!("Car {:?} has no road assigned!", entity);
            continue;
        };

        let Ok(road) = road_query.get(current_road_entity) else {
            warn!(
                "Car {:?} road entity {:?} not found!",
                entity, current_road_entity
            );
            continue;
        };

        // Get start and end intersection positions
        let Ok(start_intersection) = intersection_query.get(road.start_intersection) else {
            warn!(
                "Car {:?} start intersection {:?} not found!",
                entity, road.start_intersection
            );
            continue;
        };
        let Ok(end_intersection) = intersection_query.get(road.end_intersection) else {
            warn!(
                "Car {:?} end intersection {:?} not found!",
                entity, road.end_intersection
            );
            continue;
        };

        let start_pos = start_intersection.position;
        let end_pos = end_intersection.position;

        // Calculate road length
        let road_length = start_pos.distance(end_pos);
        if road_length < 0.01 {
            warn!(
                "Car {:?} on zero-length road (length: {:.4})",
                entity, road_length
            );
            continue; // Skip zero-length roads
        }

        // Update progress along the road
        let progress_delta = (car.speed / road_length) * time.delta_secs();
        car.progress += progress_delta;

        // If we've reached the end of the road, move to a random connected road
        if car.progress >= 1.0 {
            car.progress = 0.0;

            // Get roads connected to the end intersection
            let connected_roads = road_network.get_connected_roads(end_intersection.id);

            // Pick a random connected road (excluding the one we came from if possible)
            if !connected_roads.is_empty() {
                let next_road = if connected_roads.len() > 1 {
                    // Try to find a different road than the current one
                    let different = connected_roads
                        .iter()
                        .find(|(road_entity, _)| *road_entity != current_road_entity)
                        .or(connected_roads.first())
                        .map(|(road_entity, _)| *road_entity);
                    different
                } else {
                    info!("Car {:?}: only one road available", entity);
                    Some(connected_roads[0].0)
                };

                if let Some(next_road_entity) = next_road {
                    car.current_road_entity = Some(next_road_entity);

                    // Update rotation for the new road
                    if let Ok(new_road) = road_query.get(next_road_entity) {
                        transform.rotation = Quat::from_rotation_y(new_road.angle);
                    }
                } else {
                    warn!("Car {:?}: failed to select next road!", entity);
                }
            } else {
                warn!(
                    "Car {:?}: NO connected roads at intersection {:?}!",
                    entity, end_intersection.id
                );
            }
        } else {
            // Interpolate position along current road
            let current_pos = start_pos.lerp(end_pos, car.progress);
            transform.translation = current_pos + Vec3::new(0.0, 0.15, 0.0);
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
