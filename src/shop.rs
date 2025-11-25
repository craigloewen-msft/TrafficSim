use bevy::prelude::*;
use anyhow::Result;

use crate::car::CarArrivedAtShop;
use crate::intersection::{IntersectionEntity, spawn_intersection};
use crate::road_network::RoadNetwork;
use crate::two_way_road::spawn_two_way_road_between_intersections;

/// Component representing a shop that receives cars
#[derive(Component, Debug)]
pub struct Shop {
    /// Number of cars that have arrived at this shop
    pub cars_received: usize,
    /// Current product demand (increases over time, decreases when products arrive)
    pub product_demand: f32,
    /// Rate at which product demand increases per second
    pub product_demand_rate: f32,
}

/// Threshold at which shops want products
pub const PRODUCT_DEMAND_THRESHOLD: f32 = 10.0;
/// Amount of demand satisfied per product delivery
pub const PRODUCT_DEMAND_PER_DELIVERY: f32 = 10.0;

// Note: CarArrivedAtShop message is defined in car.rs to avoid circular dependencies

pub fn spawn_shop_intersection(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &mut ResMut<RoadNetwork>,
    position: Vec3,
) -> Result<IntersectionEntity> {
    const SHOP_SIZE: f32 = 1.2;
    let shop_color = Color::srgb(0.8, 0.4, 0.6); // Pink-ish color for shops

    let intersection_entity = spawn_intersection(
        commands,
        meshes,
        materials,
        road_network,
        position,
    )?;

    commands.entity(intersection_entity.0).insert((
        Shop { 
            cars_received: 0,
            product_demand: 10.0,
            product_demand_rate: 1.0,
        },
        Mesh3d(meshes.add(Cuboid::new(SHOP_SIZE, SHOP_SIZE, SHOP_SIZE))),
        MeshMaterial3d(materials.add(shop_color)),
    ));

    Ok(intersection_entity)
}

pub fn spawn_shop_with_driveway(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &mut ResMut<RoadNetwork>,
    shop_position: Vec3,
    road_intersection_entity: IntersectionEntity,
    road_intersection_position: Vec3,
) -> Result<IntersectionEntity> {
    let shop_intersection_entity = spawn_shop_intersection(
        commands,
        meshes,
        materials,
        road_network,
        shop_position,
    )?;

    spawn_two_way_road_between_intersections(
        commands,
        meshes,
        materials,
        road_network,
        shop_intersection_entity,
        road_intersection_entity,
        shop_position,
        road_intersection_position,
    )?;

    Ok(shop_intersection_entity)
}

/// System to handle cars arriving at shops
pub fn handle_shop_arrivals(
    mut shop_query: Query<&mut Shop>,
    mut arrival_events: MessageReader<CarArrivedAtShop>,
) {
    for event in arrival_events.read() {
        if let Ok(mut shop) = shop_query.get_mut(event.shop_entity.0) {
            shop.cars_received += 1;
            
            // Reduce product demand when goods arrive
            shop.product_demand = (shop.product_demand - PRODUCT_DEMAND_PER_DELIVERY).max(0.0);
            
            bevy::log::info!(
                "Shop {:?} received car {:?} (total received: {}, product demand now: {:.1})",
                event.shop_entity.0,
                event.car_entity,
                shop.cars_received,
                shop.product_demand
            );
            
            // Car is already despawned in the car update system
        }
    }
}

/// System to update shops - increase product demand over time
pub fn update_shops(
    mut shop_query: Query<&mut Shop>,
    time: Res<Time>,
) {
    for mut shop in shop_query.iter_mut() {
        // Increase product demand over time
        shop.product_demand += shop.product_demand_rate * time.delta_secs();
    }
}

pub struct ShopPlugin;

impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (handle_shop_arrivals, update_shops));
    }
}
