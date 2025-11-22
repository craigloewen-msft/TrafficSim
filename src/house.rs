use bevy::prelude::*;
use anyhow::Result;
use rand::seq::IndexedRandom;

use crate::car::{Car, CarEntity, spawn_car};
use crate::intersection::{Intersection, IntersectionEntity, spawn_intersection};
use crate::road::{Road};
use crate::road_network::RoadNetwork;
use crate::two_way_road::{spawn_two_way_road_between_intersections, TwoWayRoadEntity};

#[derive(Component, Debug)]
pub struct House {
    pub car: Option<CarEntity>,
}

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

/// Helper function to update a single house and spawn a car if needed
fn update_house(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &mut RoadNetwork,
    road_query: &Query<(Entity, &Road)>,
    intersection_query: &Query<(&Intersection, &Transform), Without<Car>>,
    house_entity: Entity,
    house: &mut House,
    house_entities: &[Entity],
) -> Result<()> {
    let mut rng = rand::rng();
    
    // Check if this house already has a car
    if let Some(car_entity) = house.car {
        if commands.get_entity(car_entity.0).is_err() {
            house.car = None;
        }
    }

    // If no car exists, spawn a new one
    if house.car.is_none() {
        // Find a road connected to this house intersection
        let house_intersection = IntersectionEntity(house_entity);
        
        // Get roads connected to this house from the road network
        let connected_roads = road_network
            .get_connected_roads(house_intersection)
            .ok_or_else(|| anyhow::anyhow!("House intersection not found in road network"))?;

        anyhow::ensure!(!connected_roads.is_empty(), "No roads connected to house");

        // Choose a random target house (different from current)
        let target_houses: Vec<Entity> = house_entities
            .iter()
            .copied()
            .filter(|&e| e != house_entity)
            .collect();

        let target_house = target_houses
            .choose(&mut rng)
            .ok_or_else(|| anyhow::anyhow!("No target houses available"))?;

        // Get the first road connected to this house
        let (road_entity, _next_intersection) = connected_roads
            .first()
            .ok_or_else(|| anyhow::anyhow!("No connected roads found"))?;

        // Spawn the car using the spawn_car function
        let car_entity = spawn_car(
            commands,
            meshes,
            materials,
            road_network,
            road_query,
            intersection_query,
            house_intersection,
            road_entity.0,
            *target_house,
        )?;

        // Store the car entity in the house
        house.car = Some(car_entity);
        
        bevy::log::info!("House {:?} spawned car {:?} heading to house {:?}", house_entity, car_entity.0, target_house);
    }

    Ok(())
}

pub fn update_houses(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut road_network: ResMut<RoadNetwork>,
    road_query: Query<(Entity, &Road)>,
    intersection_query: Query<(&Intersection, &Transform), Without<Car>>,
    mut house_query: Query<(Entity, &mut House, &Transform)>,
) {
    // Collect all house entities for random selection
    let house_entities: Vec<Entity> = house_query.iter().map(|(entity, _, _)| entity).collect();
    
    if house_entities.len() < 2 {
        return; // Need at least 2 houses for spawning cars
    }

    for (house_entity, mut house, _house_transform) in house_query.iter_mut() {
        if let Err(e) = update_house(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut road_network,
            &road_query,
            &intersection_query,
            house_entity,
            &mut house,
            &house_entities,
        ) {
            bevy::log::error!("Failed to update house {:?}: {:#}", house_entity, e);
        }
    }
}

pub struct HousePlugin;

impl Plugin for HousePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_houses);
    }
}
