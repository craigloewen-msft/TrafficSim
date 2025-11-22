use bevy::prelude::*;
use anyhow::Result;

use crate::car::{Car, CarEntity, spawn_car, ArrivedAtDestination};
use crate::intersection::{Intersection, IntersectionEntity, spawn_intersection};
use crate::road::{Road};
use crate::road_network::RoadNetwork;
use crate::two_way_road::{spawn_two_way_road_between_intersections, TwoWayRoadEntity};

/// Component representing a factory that receives cars and sends them back after processing
#[derive(Component, Debug)]
pub struct Factory {
    /// Cars currently being processed at this factory (car entity, return target, time remaining)
    pub processing_cars: Vec<(CarEntity, IntersectionEntity, f32)>,
}

/// Duration in seconds that a factory processes a car before sending it back
const FACTORY_PROCESSING_TIME: f32 = 2.0;

pub fn spawn_factory_intersection(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &mut ResMut<RoadNetwork>,
    position: Vec3,
) -> Result<IntersectionEntity> {
    const FACTORY_SIZE: f32 = 1.5;
    let factory_color = Color::srgb(0.5, 0.5, 0.7); // Blue-ish color for factories

    let intersection_entity = spawn_intersection(
        commands,
        meshes,
        materials,
        road_network,
        position,
    )?;

    commands.entity(intersection_entity.0).insert((
        Factory { processing_cars: Vec::new() },
        Mesh3d(meshes.add(Cuboid::new(FACTORY_SIZE, FACTORY_SIZE, FACTORY_SIZE))),
        MeshMaterial3d(materials.add(factory_color)),
    ));

    Ok(intersection_entity)
}

pub fn spawn_factory_with_driveway(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &mut ResMut<RoadNetwork>,
    factory_position: Vec3,
    road_intersection_entity: IntersectionEntity,
    road_intersection_position: Vec3,
) -> Result<IntersectionEntity> {
    let factory_intersection_entity = spawn_factory_intersection(
        commands,
        meshes,
        materials,
        road_network,
        factory_position,
    )?;

    spawn_driveway(
        commands,
        meshes,
        materials,
        road_network,
        factory_intersection_entity,
        road_intersection_entity,
        factory_position,
        road_intersection_position,
    )?;

    Ok(factory_intersection_entity)
}

fn spawn_driveway(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &mut ResMut<RoadNetwork>,
    factory_intersection: IntersectionEntity,
    road_intersection: IntersectionEntity,
    factory_pos: Vec3,
    road_pos: Vec3,
) -> Result<TwoWayRoadEntity> {
    spawn_two_way_road_between_intersections(
        commands,
        meshes,
        materials,
        road_network,
        factory_intersection,
        road_intersection,
        factory_pos,
        road_pos,
    )
}

/// System to detect cars arriving at factories
pub fn detect_car_arrivals(
    mut factory_query: Query<(Entity, &mut Factory)>,
    arrival_query: Query<(Entity, &ArrivedAtDestination)>,
) {
    for (car_entity, arrival) in arrival_query.iter() {
        // Check if the destination is a factory
        if let Ok((_, mut factory)) = factory_query.get_mut(arrival.destination.0) {
            if let Some(origin_house) = arrival.origin_house {
                // This car arrived at this factory from a house
                factory_receive_car(&mut factory, CarEntity(car_entity), origin_house);
            }
        }
    }
}

/// System to update factories - process incoming cars and spawn return cars
pub fn update_factories(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut road_network: ResMut<RoadNetwork>,
    road_query: Query<(Entity, &Road)>,
    intersection_query: Query<(&Intersection, &Transform), Without<Car>>,
    mut factory_query: Query<(Entity, &mut Factory)>,
    time: Res<Time>,
) {
    for (factory_entity, mut factory) in factory_query.iter_mut() {
        let factory_intersection = IntersectionEntity(factory_entity);
        
        // Update processing times and spawn return cars when ready
        let mut cars_to_spawn = Vec::new();
        factory.processing_cars.retain_mut(|(_car_entity, return_target, time_remaining)| {
            *time_remaining -= time.delta_secs();
            
            if *time_remaining <= 0.0 {
                // Processing complete - prepare to spawn return car
                cars_to_spawn.push(*return_target);
                false // Remove from processing list
            } else {
                true // Keep processing
            }
        });

        // Spawn return cars for completed processing
        for return_target in cars_to_spawn {
            // Find a road connected to this factory
            let connected_roads = match road_network.get_connected_roads(factory_intersection) {
                Some(roads) => roads,
                None => {
                    bevy::log::error!("Factory intersection {:?} not found in road network", factory_entity);
                    continue;
                }
            };

            if connected_roads.is_empty() {
                bevy::log::error!("No roads connected to factory {:?}", factory_entity);
                continue;
            }

            // Get the first road connected to this factory
            let (road_entity, _) = connected_roads[0];

            // Spawn a car to return to the origin house
            match spawn_car(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut road_network,
                &road_query,
                &intersection_query,
                factory_intersection,
                road_entity.0,
                return_target.0,
                None, // No origin house for return cars
            ) {
                Ok(car_entity) => {
                    bevy::log::info!(
                        "Factory {:?} spawned return car {:?} to house {:?}",
                        factory_entity,
                        car_entity.0,
                        return_target.0
                    );
                }
                Err(e) => {
                    bevy::log::error!("Failed to spawn return car from factory: {:#}", e);
                }
            }
        }

        // Check for cars that have arrived at this factory
        // We detect this by checking if any car has this factory as its final destination
        // and has reached it (we'll handle this in the car update system, but we track it here)
    }
}

/// Called when a car arrives at a factory - adds it to the processing queue
pub fn factory_receive_car(
    factory: &mut Factory,
    car_entity: CarEntity,
    origin_house: IntersectionEntity,
) {
    bevy::log::info!(
        "Factory received car {:?} from house {:?}, will process for {} seconds",
        car_entity.0,
        origin_house.0,
        FACTORY_PROCESSING_TIME
    );
    
    factory.processing_cars.push((car_entity, origin_house, FACTORY_PROCESSING_TIME));
}

pub struct FactoryPlugin;

impl Plugin for FactoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (detect_car_arrivals, update_factories));
    }
}
