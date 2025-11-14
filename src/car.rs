use crate::intersection::{Intersection, IntersectionEntity};
use crate::road::Road;
use crate::road_network::RoadNetwork;
use bevy::log::{info, warn};
use bevy::prelude::*;
use rand::seq::{IndexedRandom, IteratorRandom};

/// Component that marks an entity as a car
#[derive(Component)]
pub struct Car {
    pub speed: f32,
    pub max_speed: f32,
    pub current_road_entity: Option<Entity>, // The road entity the car is currently on
    pub progress: f32,                       // 0.0 to 1.0 along the current road
    pub start_intersection: Option<Entity>,  // The intersection where we started on this road
    pub target_intersection: Option<Entity>, // The intersection we're traveling toward
    pub final_target_intersection: Option<Entity>, // The final destination intersection
    pub path: Vec<IntersectionEntity>, // Path of intersection entities to follow to reach the final destination
}

impl Default for Car {
    fn default() -> Self {
        Self {
            speed: 4.0,
            max_speed: 5.0,
            current_road_entity: None,
            progress: 0.0,
            start_intersection: None,
            target_intersection: None,
            final_target_intersection: None,
            path: Vec::new(),
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
        // Spawn a car on a random road
        let Some(&road_entity) = road_entities.choose(&mut rng) else {
            warn!("Failed to choose random road!");
            continue;
        };

        if let Ok((_, road)) = road_query.get(road_entity) {
            // Get the start intersection position
            if let Ok(start_intersection) = intersection_query.get(road.start_intersection_entity.0) {
                let spawn_pos = start_intersection.position + Vec3::new(0.0, 0.3, 0.0);
                info!("Spawning car at position: {:.2?}", spawn_pos);

                // Choose a random final destination intersection (different from start)
                let start_entity = road.start_intersection_entity.0;

                let final_destination = all_intersections
                    .iter()
                    .filter(|&&entity| entity != start_entity)
                    .choose(&mut rng);

                // Set up the final destination and compute path
                let (final_target_entity, path) = match final_destination {
                    Some(&entity) => {
                        if let Ok(destination_intersection) = intersection_query.get(entity) {
                            info!(
                                "Car final destination: intersection at position {:.2?}",
                                destination_intersection.position
                            );

                            let path = road_network.find_path(
                                crate::intersection::IntersectionEntity(start_entity),
                                crate::intersection::IntersectionEntity(entity)
                            ).unwrap_or_else(|| {
                                warn!("No path found from start to destination");
                                Vec::new()
                            });

                            (Some(entity), path)
                        } else {
                            warn!("Could not query destination intersection!");
                            (None, Vec::new())
                        }
                    }
                    None => {
                        warn!("Could not find valid final destination!");
                        (None, Vec::new())
                    }
                };

                commands.spawn(CarBundle {
                    car: Car {
                        current_road_entity: Some(road_entity),
                        progress: 0.0,
                        start_intersection: Some(road.start_intersection_entity.0),
                        target_intersection: Some(road.end_intersection_entity.0),
                        final_target_intersection: final_target_entity,
                        path,
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
                    road.start_intersection_entity.0
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
    road_query: Query<(Entity, &Road)>,
    intersection_query: Query<&Intersection>,
    mut car_query: Query<(Entity, &mut Car, &mut Transform)>,
) {
    for (entity, mut car, mut transform) in car_query.iter_mut() {
        // debug!("Car {:?}: progress={:.3}, speed={:.2}, pos={:.2?}",
        //        entity, car.progress, car.speed, transform.translation);

        // Check if we've reached the final destination
        if let Some(final_target_entity) = car.final_target_intersection {
            if let Some(target_entity) = car.target_intersection {
                if target_entity == final_target_entity && car.progress >= 1.0 {
                    // Stop the car - just continue to the next iteration
                    continue;
                }
            }
        }

        // Get the current road the car is on
        let Some(current_road_entity) = car.current_road_entity else {
            warn!("Car {:?} has no road assigned!", entity);
            continue;
        };

        let Ok(_road) = road_query.get(current_road_entity) else {
            warn!(
                "Car {:?} road entity {:?} not found!",
                entity, current_road_entity
            );
            continue;
        };

        // Get start and end intersection positions based on car's travel direction
        let Some(start_entity) = car.start_intersection else {
            warn!("Car {:?} has no start intersection!", entity);
            continue;
        };
        let Some(target_entity) = car.target_intersection else {
            warn!("Car {:?} has no target intersection!", entity);
            continue;
        };

        let Ok(start_intersection) = intersection_query.get(start_entity) else {
            warn!(
                "Car {:?} start intersection {:?} not found!",
                entity, start_entity
            );
            continue;
        };
        let Ok(target_intersection) = intersection_query.get(target_entity) else {
            warn!(
                "Car {:?} target intersection {:?} not found!",
                entity, target_entity
            );
            continue;
        };

        let start_pos = start_intersection.position;
        let end_pos = target_intersection.position;

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

        // Check if we've reached the end of the current road
        if car.progress >= 1.0 {
            // We've reached the target intersection
            // Check if this is our final destination
            if Some(target_entity) == car.final_target_intersection {
                info!(
                    "Car {:?} reached final destination at intersection",
                    entity
                );
                car.progress = 1.0; // Clamp progress
                transform.translation = end_pos;
                continue; // Stop moving
            }

            // Throw an error if path is not available
            if car.path.is_empty() {
                error!(
                    "Car {:?} has no path to follow from current intersection!",
                    entity
                );
                continue;
            }

            // Get the next intersection entity from the path
            let next_intersection_entity = car.path.remove(0);
            
            // Find the road that connects current intersection to next intersection
            let Some(next_road_entity) = road_network.find_road_between(
                crate::intersection::IntersectionEntity(target_entity),
                next_intersection_entity
            ) else {
                error!(
                    "Car {:?}: no road found between current intersection and next intersection!",
                    entity
                );
                continue;
            };
            
            car.current_road_entity = Some(next_road_entity.0);
            car.progress = 0.0;

            // Determine which end of the new road we're at and set our direction
            if let Ok((_, new_road)) = road_query.get(next_road_entity.0) {
                // Figure out which direction to travel on the new road
                if new_road.start_intersection_entity.0 == target_entity {
                    // We're at the start, so travel toward the end
                    car.start_intersection = Some(new_road.start_intersection_entity.0);
                    car.target_intersection = Some(new_road.end_intersection_entity.0);
                    transform.rotation = Quat::from_rotation_y(new_road.angle);
                } else if new_road.end_intersection_entity.0 == target_entity {
                    // We're at the end, so travel toward the start
                    car.start_intersection = Some(new_road.end_intersection_entity.0);
                    car.target_intersection = Some(new_road.start_intersection_entity.0);
                    // Rotate 180 degrees to face the opposite direction
                    transform.rotation =
                        Quat::from_rotation_y(new_road.angle + std::f32::consts::PI);
                } else {
                    error!(
                        "Car {:?}: new road doesn't connect to current intersection!",
                        entity
                    );
                    car.start_intersection = Some(new_road.start_intersection_entity.0);
                    car.target_intersection = Some(new_road.end_intersection_entity.0);
                }
            } else {
                error!("Car {:?}: failed to query next road from path!", entity);
            }
        } else {
            // Interpolate position along current road
            let translate_vector = start_pos.lerp(end_pos, car.progress);
            transform.translation = translate_vector;
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
