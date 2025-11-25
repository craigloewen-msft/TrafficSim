use bevy::prelude::*;
use anyhow::Result;
use rand::Rng;

use crate::car::{Car, CarEntity, spawn_car};
use crate::factory::{Factory, LABOR_DEMAND_THRESHOLD, try_reserve_worker};
use crate::intersection::{Intersection, IntersectionEntity, spawn_intersection};
use crate::road::{Road};
use crate::road_network::RoadNetwork;
use crate::two_way_road::{spawn_two_way_road_between_intersections, TwoWayRoadEntity};

#[derive(Component, Debug)]
pub struct House {
    pub car: Option<CarEntity>,
}

/// Component to mark the visual demand indicator entity
#[derive(Component, Debug)]
pub struct DemandIndicator;

pub fn spawn_house_intersection(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &mut ResMut<RoadNetwork>,
    position: Vec3,
) -> Result<IntersectionEntity> {
    const HOUSE_SIZE: f32 = 1.0;
    let house_color = Color::srgb(0.7, 0.6, 0.4);

    let intersection_entity = spawn_intersection(
        commands,
        meshes,
        materials,
        road_network,
        position,
    )?;

    commands.entity(intersection_entity.0).insert((
        House { car: None },
        Mesh3d(meshes.add(Cuboid::new(HOUSE_SIZE, HOUSE_SIZE, HOUSE_SIZE))),
        MeshMaterial3d(materials.add(house_color)),
    ));

    // Spawn demand indicator above the house
    let indicator_entity = commands.spawn((
        DemandIndicator,
        Mesh3d(meshes.add(Sphere::new(0.2))),
        MeshMaterial3d(materials.add(Color::srgb(0.0, 1.0, 0.0))),
        Transform::from_translation(Vec3::new(0.0, 1.2, 0.0)),
    )).id();
    
    commands.entity(intersection_entity.0).add_child(indicator_entity);

    Ok(intersection_entity)
}

pub fn spawn_house_with_driveway(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &mut ResMut<RoadNetwork>,
    house_position: Vec3,
    road_intersection_entity: IntersectionEntity,
    road_intersection_position: Vec3,
) -> Result<IntersectionEntity> {
    let house_intersection_entity = spawn_house_intersection(
        commands,
        meshes,
        materials,
        road_network,
        house_position,
    )?;

    spawn_driveway(
        commands,
        meshes,
        materials,
        road_network,
        house_intersection_entity,
        road_intersection_entity,
        house_position,
        road_intersection_position,
    )?;

    Ok(house_intersection_entity)
}

fn spawn_driveway(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &mut ResMut<RoadNetwork>,
    house_intersection: IntersectionEntity,
    road_intersection: IntersectionEntity,
    house_pos: Vec3,
    road_pos: Vec3,
) -> Result<TwoWayRoadEntity> {
    // Use spawn_two_way_road_between_intersections to create a bidirectional driveway
    // This uses existing intersections and creates one visual mesh and two logical roads
    spawn_two_way_road_between_intersections(
        commands,
        meshes,
        materials,
        road_network,
        house_intersection,
        road_intersection,
        house_pos,
        road_pos,
    )
}

/// System to spawn workers from houses to factories with demand
pub fn spawn_workers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut road_network: ResMut<RoadNetwork>,
    road_query: Query<(Entity, &Road)>,
    intersection_query: Query<(&Intersection, &Transform), Without<Car>>,
    mut house_query: Query<(Entity, &mut House)>,
    mut factory_query: Query<(Entity, &mut Factory)>,
) {
    // Collect factories with high labor demand
    let factories_with_demand: Vec<Entity> = factory_query
        .iter()
        .filter(|(_, factory)| factory.labor_demand >= LABOR_DEMAND_THRESHOLD)
        .map(|(entity, _)| entity)
        .collect();
    
    if factories_with_demand.is_empty() {
        return; // No factories need workers
    }

    for (house_entity, mut house) in house_query.iter_mut() {
        // Clean up car reference if car was despawned
        if let Some(car_entity) = house.car {
            if commands.get_entity(car_entity.0).is_err() {
                house.car = None;
            }
        }
        
        // Only spawn if this house doesn't have a car out
        if house.car.is_none() {
            // Choose a random factory from those with high demand
            let factory_index = rand::rng().random_range(0..factories_with_demand.len());
            let factory_entity = factories_with_demand[factory_index];
            
            // Try to reserve a worker slot at this factory
            let Ok((_, mut factory)) = factory_query.get_mut(factory_entity) else {
                continue;
            };
            
            if !try_reserve_worker(&mut factory) {
                // Factory no longer needs workers
                continue;
            }
            
            // Factory accepted! Now spawn the car
            let house_intersection = IntersectionEntity(house_entity);
            
            let Some(connected_roads) = road_network.get_connected_roads(house_intersection) else {
                bevy::log::error!("House intersection {:?} not found in road network", house_entity);
                continue;
            };

            if connected_roads.is_empty() {
                bevy::log::error!("No roads connected to house {:?}", house_entity);
                continue;
            }

            let (road_entity, _) = connected_roads[0];

            match spawn_car(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut road_network,
                &road_query,
                &intersection_query,
                house_intersection,
                road_entity.0,
                factory_entity,
                Some(house_intersection),
            ) {
                Ok(car_entity) => {
                    house.car = Some(car_entity);
                    bevy::log::info!(
                        "House {:?} spawned car {:?} heading to factory {:?}",
                        house_entity,
                        car_entity.0,
                        factory_entity
                    );
                }
                Err(e) => {
                    bevy::log::error!("Failed to spawn car from house {:?}: {:#}", house_entity, e);
                }
            }
        }
    }
}

/// System to update demand indicators for houses
pub fn update_house_demand_indicators(
    house_query: Query<&Children, With<House>>,
    mut indicator_query: Query<&mut MeshMaterial3d<StandardMaterial>, With<DemandIndicator>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for children in house_query.iter() {
        for child in children.iter() {
            if let Ok(material_handle) = indicator_query.get_mut(child) {
                // Houses without cars show green, houses with cars out show gray
                // (We can't easily check if car is out here without adding complexity)
                // For now, just show green as "ready to send workers"
                if let Some(material) = materials.get_mut(&material_handle.0) {
                    material.base_color = Color::srgb(0.0, 1.0, 0.0);
                }
            }
        }
    }
}

pub struct HousePlugin;

impl Plugin for HousePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (spawn_workers, update_house_demand_indicators));
    }
}
