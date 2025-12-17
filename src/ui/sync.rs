//! Systems for syncing Bevy entities with simulation state

use bevy::prelude::*;

use super::components::{
    CarLink, DeliveryIndicator, DemandIndicator, EntityMappings, FactoryLink, HouseLink, ShopLink,
    SimSynced, SimWorldResource,
};
use crate::{
    simulation::{CarId, VehicleType, GOAL_DELIVERIES, GOAL_MONEY},
    ui::components::GlobalDemandText,
};

/// System to run simulation tick
pub fn tick_simulation(time: Res<Time>, mut sim_world: ResMut<SimWorldResource>) {
    sim_world.0.tick(time.delta_secs());
}

/// System to sync car visuals from simulation state
pub fn sync_cars(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sim_world: Res<SimWorldResource>,
    mut mappings: ResMut<EntityMappings>,
    mut car_query: Query<(Entity, &CarLink, &mut Transform)>,
) {
    let world = &sim_world.0;
    const CAR_LENGTH: f32 = 0.5;
    const TRUCK_LENGTH: f32 = 0.8;

    // Update existing cars and track which ones still exist
    let mut existing_car_ids: std::collections::HashSet<CarId> = std::collections::HashSet::new();

    for (entity, link, mut transform) in car_query.iter_mut() {
        if let Some(car) = world.cars.get(&link.0) {
            existing_car_ids.insert(link.0);
            let y_height = match car.vehicle_type {
                VehicleType::Car => 0.3,
                VehicleType::Truck => 0.4,
            };
            transform.translation = Vec3::new(car.position.x, y_height, car.position.z);
            transform.rotation = Quat::from_rotation_y(car.angle);
        } else {
            // Car no longer exists in simulation, despawn
            commands.entity(entity).despawn();
            mappings.cars.remove(&link.0);
        }
    }

    // Spawn new cars/trucks
    for (id, car) in &world.cars {
        if !existing_car_ids.contains(id) {
            let (width, height, length, color, y_height) = match car.vehicle_type {
                VehicleType::Car => (0.3, 0.2, CAR_LENGTH, Color::srgb(0.8, 0.2, 0.2), 0.3),
                VehicleType::Truck => (0.4, 0.35, TRUCK_LENGTH, Color::srgb(0.2, 0.4, 0.8), 0.4),
            };

            let entity = commands
                .spawn((
                    SimSynced,
                    CarLink(*id),
                    Mesh3d(meshes.add(Cuboid::new(width, height, length))),
                    MeshMaterial3d(materials.add(color)),
                    Transform::from_translation(Vec3::new(
                        car.position.x,
                        y_height,
                        car.position.z,
                    ))
                    .with_rotation(Quat::from_rotation_y(car.angle)),
                ))
                .id();
            mappings.cars.insert(*id, entity);
        }
    }
}

/// System to update factory demand indicators
pub fn update_factory_indicators(
    sim_world: Res<SimWorldResource>,
    factory_query: Query<(&FactoryLink, &Children)>,
    mut indicator_query: Query<&mut MeshMaterial3d<StandardMaterial>, With<DemandIndicator>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (link, children) in factory_query.iter() {
        if let Some(factory) = sim_world.0.factories.get(&link.0) {
            for child in children.iter() {
                if let Ok(material_handle) = indicator_query.get_mut(child) {
                    if let Some(material) = materials.get_mut(&material_handle.0) {
                        // Red if truck is out (busy), green if truck is home (available)
                        if factory.truck.is_some() {
                            material.base_color = Color::srgb(1.0, 0.0, 0.0); // Red - busy
                        } else {
                            material.base_color = Color::srgb(0.0, 1.0, 0.0); // Green - available
                        }
                    }
                }
            }
        }
    }
}

/// System to update house demand indicators
pub fn update_house_indicators(
    sim_world: Res<SimWorldResource>,
    house_query: Query<(&HouseLink, &Children)>,
    mut indicator_query: Query<&mut MeshMaterial3d<StandardMaterial>, With<DemandIndicator>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (link, children) in house_query.iter() {
        if let Some(house) = sim_world.0.houses.get(&link.0) {
            for child in children.iter() {
                if let Ok(material_handle) = indicator_query.get_mut(child) {
                    if let Some(material) = materials.get_mut(&material_handle.0) {
                        // Red if car is out (busy), green if car is home (available)
                        if house.car.is_some() {
                            material.base_color = Color::srgb(1.0, 0.0, 0.0); // Red - busy
                        } else {
                            material.base_color = Color::srgb(0.0, 1.0, 0.0); // Green - available
                        }
                    }
                }
            }
        }
    }
}

/// System to update shop demand indicators
pub fn update_shop_indicators(
    sim_world: Res<SimWorldResource>,
    shop_query: Query<(&ShopLink, &Children)>,
    mut indicator_query: Query<&mut MeshMaterial3d<StandardMaterial>, With<DemandIndicator>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Shops are passive - just show green always (they just receive deliveries)
    for (link, children) in shop_query.iter() {
        if sim_world.0.shops.get(&link.0).is_some() {
            for child in children.iter() {
                if let Ok(material_handle) = indicator_query.get_mut(child) {
                    if let Some(material) = materials.get_mut(&material_handle.0) {
                        material.base_color = Color::srgb(0.0, 1.0, 0.0); // Green - always ready
                    }
                }
            }
        }
    }
}

/// System to update global demand text in the UI toolbar
pub fn update_global_demand_text(
    sim_world: Res<SimWorldResource>,
    mut text_query: Query<(&GlobalDemandText, &mut Text)>,
) {
    let demand = sim_world.0.calculate_global_demand();

    for (demand_type, mut text) in text_query.iter_mut() {
        match demand_type {
            GlobalDemandText::FactoriesWaiting => {
                **text = format!(
                    "Factories Busy: {}/{}",
                    demand.factories_waiting, demand.total_factories
                );
            }
            GlobalDemandText::ShopsWaiting => {
                **text = format!("Shops: {}", demand.total_shops);
            }
            GlobalDemandText::HousesWaiting => {
                **text = format!(
                    "Houses Busy: {}/{}",
                    demand.houses_waiting, demand.total_houses
                );
            }
            GlobalDemandText::Money => {
                if let Some(game_state) = &sim_world.0.game_state {
                    **text = format!("Money: ${}", game_state.money);
                } else {
                    **text = "Money: N/A".to_string();
                }
            }
            GlobalDemandText::WorkerTrips => {
                if let Some(game_state) = &sim_world.0.game_state {
                    **text = format!("Worker Trips: {}", game_state.worker_trips_completed);
                } else {
                    **text = "Worker Trips: N/A".to_string();
                }
            }
            GlobalDemandText::ShopDeliveries => {
                if let Some(game_state) = &sim_world.0.game_state {
                    **text = format!(
                        "Shop Deliveries: {} / {}",
                        game_state.shop_deliveries_completed, GOAL_DELIVERIES
                    );
                } else {
                    **text = "Shop Deliveries: N/A".to_string();
                }
            }
            GlobalDemandText::GoalStatus => {
                if let Some(game_state) = &sim_world.0.game_state {
                    if game_state.is_won {
                        **text = "🎉 YOU WIN! Goal Complete! 🎉".to_string();
                    } else if game_state.is_lost {
                        **text = "💀 BANKRUPT - Game Over 💀".to_string();
                    } else {
                        **text = format!("Goal: {} deliveries OR ${}", GOAL_DELIVERIES, GOAL_MONEY);
                    }
                } else {
                    **text = "Goal: N/A".to_string();
                }
            }
        }
    }
}

/// System to update factory delivery indicators
pub fn update_factory_delivery_indicators(
    sim_world: Res<SimWorldResource>,
    factory_query: Query<(&FactoryLink, &Children)>,
    mut indicator_query: Query<&mut MeshMaterial3d<StandardMaterial>, With<DeliveryIndicator>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const DELIVERY_INDICATOR_ACTIVE_COLOR: Color = Color::srgb(1.0, 0.8, 0.0); // Gold/yellow
    const DELIVERY_INDICATOR_EMPTY_COLOR: Color = Color::srgb(0.3, 0.3, 0.3); // Dark gray

    for (link, children) in factory_query.iter() {
        if let Some(factory) = sim_world.0.factories.get(&link.0) {
            // Iterate over delivery indicator children (query filters for DeliveryIndicator component)
            let mut indicator_index = 0;
            for child in children.iter() {
                if let Ok(mut material_handle) = indicator_query.get_mut(child) {
                    if let Some(material) = materials.get_mut(&material_handle.0) {
                        // Light up indicators based on deliveries_ready count
                        if indicator_index < factory.deliveries_ready as usize {
                            material.base_color = DELIVERY_INDICATOR_ACTIVE_COLOR;
                        } else {
                            material.base_color = DELIVERY_INDICATOR_EMPTY_COLOR;
                        }
                        indicator_index += 1;
                    }
                }
            }
        }
    }
}
