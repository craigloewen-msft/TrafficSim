use bevy::prelude::*;
use anyhow::Result;
use rand::seq::IndexedRandom;

use crate::car::{Car, CarEntity, spawn_car, CarArrivedAtFactory};
use crate::house::DemandIndicator;
use crate::intersection::{Intersection, IntersectionEntity, spawn_intersection};
use crate::road::{Road};
use crate::road_network::RoadNetwork;
use crate::two_way_road::{spawn_two_way_road_between_intersections, TwoWayRoadEntity};
use crate::shop::{Shop, PRODUCT_DEMAND_THRESHOLD};

/// Component representing a factory that receives cars and sends them to shops after processing
#[derive(Component, Debug)]
pub struct Factory {
    /// Cars currently being processed at this factory (car entity, shop target, time remaining)
    pub processing_cars: Vec<(CarEntity, IntersectionEntity, f32)>,
    /// Current labor demand (increases over time, decreases when workers arrive)
    pub labor_demand: f32,
    /// Rate at which labor demand increases per second
    pub labor_demand_rate: f32,
    /// Current inventory of produced goods
    pub inventory: u32,
    /// Maximum inventory capacity
    pub max_inventory: u32,
}

/// Duration in seconds that a factory processes a car before sending it back
const FACTORY_PROCESSING_TIME: f32 = 2.0;
/// Threshold at which factories need workers
pub const LABOR_DEMAND_THRESHOLD: f32 = 10.0;
/// Amount of demand reduced when a worker arrives
const LABOR_DEMAND_PER_WORKER: f32 = 10.0;

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
        Factory { 
            processing_cars: Vec::new(),
            labor_demand: 10.0,
            labor_demand_rate: 1.0,
            inventory: 0,
            max_inventory: 10,
        },
        Mesh3d(meshes.add(Cuboid::new(FACTORY_SIZE, FACTORY_SIZE, FACTORY_SIZE))),
        MeshMaterial3d(materials.add(factory_color)),
    ));

    // Spawn demand indicator above the factory
    let indicator_entity = commands.spawn((
        DemandIndicator,
        Mesh3d(meshes.add(Sphere::new(0.25))),
        MeshMaterial3d(materials.add(Color::srgb(0.0, 1.0, 0.0))),
        Transform::from_translation(Vec3::new(0.0, 1.5, 0.0)),
    )).id();
    
    commands.entity(intersection_entity.0).add_child(indicator_entity);

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
    mut arrival_events: MessageReader<CarArrivedAtFactory>,
    shop_query: Query<Entity, With<crate::shop::Shop>>,
) {
    // Collect all shop entities
    let shop_entities: Vec<Entity> = shop_query.iter().collect();
    
    if shop_entities.is_empty() {
        bevy::log::warn!("No shops available for factory to send cars to");
        return;
    }

    for event in arrival_events.read() {
        // Check if the destination is a factory
        if let Ok((_, mut factory)) = factory_query.get_mut(event.factory_entity.0) {
            // Choose a random shop as the target
            let mut rng = rand::rng();
            if let Some(&shop_entity) = shop_entities.choose(&mut rng) {
                factory_receive_car(&mut factory, CarEntity(event.car_entity), IntersectionEntity(shop_entity));
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
    shop_query: Query<(Entity, &Shop)>,
    time: Res<Time>,
) {
    for (factory_entity, mut factory) in factory_query.iter_mut() {
        let factory_intersection = IntersectionEntity(factory_entity);
        
        // Increase labor demand over time
        factory.labor_demand += factory.labor_demand_rate * time.delta_secs();
        
        // Update processing times and add to inventory when ready
        let mut products_produced = 0;
        factory.processing_cars.retain_mut(|(_car_entity, _shop_target, time_remaining)| {
            *time_remaining -= time.delta_secs();
            
            if *time_remaining <= 0.0 {
                // Processing complete - add to inventory
                products_produced += 1;
                false // Remove from processing list
            } else {
                true // Keep processing
            }
        });

        // Add produced goods to inventory (up to max capacity)
        if products_produced > 0 {
            let space_available = factory.max_inventory.saturating_sub(factory.inventory);
            let to_add = products_produced.min(space_available);
            factory.inventory += to_add;
            
            bevy::log::info!(
                "Factory {:?} produced {} goods, inventory now {}/{}",
                factory_entity,
                to_add,
                factory.inventory,
                factory.max_inventory
            );
        }

        // Check shops with high demand and send inventory
        let mut shops_needing_products: Vec<Entity> = shop_query
            .iter()
            .filter(|(_, shop)| shop.product_demand >= PRODUCT_DEMAND_THRESHOLD)
            .map(|(entity, _)| entity)
            .collect();

        // Send products to shops if we have inventory
        while factory.inventory > 0 && !shops_needing_products.is_empty() {
            // Get a shop that needs products
            let shop_entity = shops_needing_products.pop().unwrap();
            let shop_target = IntersectionEntity(shop_entity);
            
            // Find a road connected to this factory
            let connected_roads = match road_network.get_connected_roads(factory_intersection) {
                Some(roads) => roads,
                None => {
                    bevy::log::error!("Factory intersection {:?} not found in road network", factory_entity);
                    break;
                }
            };

            if connected_roads.is_empty() {
                bevy::log::error!("No roads connected to factory {:?}", factory_entity);
                break;
            }

            // Get the first road connected to this factory
            let (road_entity, _) = connected_roads[0];

            // Spawn a car to go to the shop with the product
            match spawn_car(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut road_network,
                &road_query,
                &intersection_query,
                factory_intersection,
                road_entity.0,
                shop_target.0,
                None, // No origin house for factory->shop cars
            ) {
                Ok(car_entity) => {
                    factory.inventory -= 1; // Deduct from inventory
                    bevy::log::info!(
                        "Factory {:?} spawned delivery car {:?} to shop {:?} (inventory: {}/{})",
                        factory_entity,
                        car_entity.0,
                        shop_target.0,
                        factory.inventory,
                        factory.max_inventory
                    );
                }
                Err(e) => {
                    bevy::log::error!("Failed to spawn car from factory to shop: {:#}", e);
                    break;
                }
            }
        }

        // Note: Cars arriving at this factory are handled in detect_car_arrivals system
    }
}

/// Called when a car arrives at a factory - adds it to the processing queue
pub fn factory_receive_car(
    factory: &mut Factory,
    car_entity: CarEntity,
    shop_target: IntersectionEntity,
) {
    // Reduce labor demand when worker arrives
    factory.labor_demand = (factory.labor_demand - LABOR_DEMAND_PER_WORKER).max(0.0);
    
    bevy::log::info!(
        "Factory received car {:?}, will process for {} seconds and send to shop {:?} (labor demand now: {:.1})",
        car_entity.0,
        FACTORY_PROCESSING_TIME,
        shop_target.0,
        factory.labor_demand
    );
    
    factory.processing_cars.push((car_entity, shop_target, FACTORY_PROCESSING_TIME));
}

/// Try to reserve a worker slot at this factory. Returns true if accepted (had high demand), false otherwise.
/// If accepted, reduces the factory's labor demand.
pub fn try_reserve_worker(factory: &mut Factory) -> bool {
    if factory.labor_demand >= LABOR_DEMAND_THRESHOLD {
        // Reserve the slot by reducing demand now
        factory.labor_demand = (factory.labor_demand - LABOR_DEMAND_PER_WORKER).max(0.0);
        true
    } else {
        false
    }
}

/// System to update demand indicators for factories
pub fn update_factory_demand_indicators(
    factory_query: Query<(&Factory, &Children)>,
    mut indicator_query: Query<&mut MeshMaterial3d<StandardMaterial>, With<DemandIndicator>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (factory, children) in factory_query.iter() {
        for child in children.iter() {
            if let Ok(material_handle) = indicator_query.get_mut(child) {
                if let Some(material) = materials.get_mut(&material_handle.0) {
                    // Color based on labor demand: green (low) -> yellow -> red (high)
                    let demand_ratio = (factory.labor_demand / (LABOR_DEMAND_THRESHOLD * 2.0)).min(1.0);
                    
                    if demand_ratio < 0.5 {
                        // Green to yellow transition
                        let t = demand_ratio * 2.0;
                        material.base_color = Color::srgb(t, 1.0, 0.0);
                    } else {
                        // Yellow to red transition
                        let t = (demand_ratio - 0.5) * 2.0;
                        material.base_color = Color::srgb(1.0, 1.0 - t, 0.0);
                    }
                }
            }
        }
    }
}

pub struct FactoryPlugin;

impl Plugin for FactoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (detect_car_arrivals, update_factories, update_factory_demand_indicators));
    }
}
