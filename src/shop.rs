use bevy::prelude::*;
use anyhow::Result;

use crate::car::CarArrivedAtShop;
use crate::house::DemandIndicator;
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

    // Spawn demand indicator above the shop
    let indicator_entity = commands.spawn((
        DemandIndicator,
        Mesh3d(meshes.add(Sphere::new(0.22))),
        MeshMaterial3d(materials.add(Color::srgb(0.0, 1.0, 0.0))),
        Transform::from_translation(Vec3::new(0.0, 1.3, 0.0)),
    )).id();
    
    commands.entity(intersection_entity.0).add_child(indicator_entity);

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

/// System to update demand indicators for shops
pub fn update_shop_demand_indicators(
    shop_query: Query<(&Shop, &Children)>,
    mut indicator_query: Query<&mut MeshMaterial3d<StandardMaterial>, With<DemandIndicator>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (shop, children) in shop_query.iter() {
        for child in children.iter() {
            if let Ok(material_handle) = indicator_query.get_mut(child) {
                if let Some(material) = materials.get_mut(&material_handle.0) {
                    // Color based on product demand: green (low) -> yellow -> red (high)
                    let demand_ratio = (shop.product_demand / (PRODUCT_DEMAND_THRESHOLD * 2.0)).min(1.0);
                    
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

pub struct ShopPlugin;

impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (handle_shop_arrivals, update_shops, update_shop_demand_indicators));
    }
}
