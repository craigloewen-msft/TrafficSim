//! Car spawning and management for the traffic simulation
//!
//! This module contains functions for spawning, despawning, and updating vehicles.
//! It separates car management logic from the main world coordination.

use anyhow::{Context, Result};
use ordered_float::OrderedFloat;
use std::collections::HashMap;

use super::building::{SimApartment, SimFactory};
use super::car::{CarUpdateResult, SimCar};
use super::intersection::SimIntersection;
use super::road_network::SimRoadNetwork;
use super::types::{ApartmentId, CarId, FactoryId, IntersectionId, TripType, VehicleType};

/// Spawn a vehicle from a given intersection to a destination
///
/// # Arguments
/// * `car_id` - The pre-generated car ID
/// * `from_intersection` - The starting intersection
/// * `to_intersection` - The destination intersection
/// * `vehicle_type` - The type of vehicle (Car or Truck)
/// * `trip_type` - The type of trip (Outbound or Return)
/// * `origin_apartment` - The apartment this car belongs to (for cars)
/// * `origin_factory` - The factory this truck belongs to (for trucks)
/// * `road_network` - The road network to use for pathfinding
/// * `speed` - The speed of the vehicle
///
/// Returns the new car if successful
#[allow(clippy::too_many_arguments)]
pub fn spawn_vehicle(
    car_id: CarId,
    from_intersection: IntersectionId,
    to_intersection: IntersectionId,
    vehicle_type: VehicleType,
    trip_type: TripType,
    origin_apartment: Option<ApartmentId>,
    origin_factory: Option<FactoryId>,
    road_network: &mut SimRoadNetwork,
    speed: f32,
) -> Result<SimCar> {
    // Find connected roads from the starting intersection
    let connected_roads = road_network
        .get_connected_roads(from_intersection)
        .context("Starting intersection not found in road network")?;

    if connected_roads.is_empty() {
        anyhow::bail!("No roads connected to starting intersection");
    }

    // Find the path
    let path = road_network
        .find_path(from_intersection, to_intersection)
        .context("No path found to destination")?;

    if path.is_empty() && from_intersection != to_intersection {
        anyhow::bail!("Empty path but different start/end");
    }

    // Get the first road in the path
    let first_target = path.first().copied().unwrap_or(to_intersection);
    let road_id = road_network
        .find_road_between(from_intersection, first_target)
        .context("No road to first path intersection")?;

    let road = road_network
        .get_road(road_id)
        .context("Road not found")?;

    let road_angle = road.angle;

    let start_pos = *road_network
        .get_intersection_position(from_intersection)
        .context("Start intersection position not found")?;

    let car = SimCar::new(
        car_id,
        speed,
        road_id,
        from_intersection,
        path,
        start_pos,
        road_angle,
        vehicle_type,
        trip_type,
        origin_apartment,
        origin_factory,
    );

    // Register car on road
    road_network.update_car_road_position(
        car_id,
        road_id,
        OrderedFloat(0.0),
        false,
        None,
        OrderedFloat(0.0),
    )?;

    Ok(car)
}

/// Despawn a car and clean up references
///
/// # Arguments
/// * `car_id` - The ID of the car to despawn
/// * `cars` - The cars collection
/// * `road_network` - The road network for tracking cleanup
/// * `apartments` - The apartments collection for reference cleanup
/// * `factories` - The factories collection for reference cleanup
pub fn despawn_car(
    car_id: CarId,
    cars: &mut HashMap<CarId, SimCar>,
    road_network: &mut SimRoadNetwork,
    apartments: &mut HashMap<ApartmentId, SimApartment>,
    factories: &mut HashMap<FactoryId, SimFactory>,
) {
    // Get car info before removing
    let car_info = cars
        .get(&car_id)
        .map(|c| (c.origin_apartment, c.origin_factory));

    cars.remove(&car_id);
    road_network.remove_car_from_tracking(car_id);

    if let Some((origin_apartment, origin_factory)) = car_info {
        // Clear apartment car reference
        if let Some(apartment_id) = origin_apartment {
            if let Some(apartment) = apartments.get_mut(&apartment_id) {
                for car_slot in &mut apartment.cars {
                    if *car_slot == Some(car_id) {
                        *car_slot = None;
                        break;
                    }
                }
            }
        }

        // Clear factory truck reference
        if let Some(factory_id) = origin_factory {
            if let Some(factory) = factories.get_mut(&factory_id) {
                if factory.truck == Some(car_id) {
                    factory.truck = None;
                }
            }
        }
    }
}

/// Update all cars in the simulation
///
/// Returns a list of (car_id, result) tuples for cars that need special handling
pub fn update_cars(
    delta_secs: f32,
    cars: &mut HashMap<CarId, SimCar>,
    road_network: &mut SimRoadNetwork,
    intersections: &mut HashMap<IntersectionId, SimIntersection>,
) -> Vec<(CarId, CarUpdateResult)> {
    let mut results = Vec::new();

    // Collect car IDs to avoid borrow issues
    let car_ids: Vec<CarId> = cars.keys().copied().collect();

    for car_id in car_ids {
        // Get car mutably, update it, then process result
        if let Some(mut car) = cars.remove(&car_id) {
            let result = car.update(delta_secs, road_network, intersections);

            match result {
                Ok(CarUpdateResult::Continue) => {
                    cars.insert(car_id, car);
                }
                Ok(CarUpdateResult::Despawn) => {
                    // Put car back temporarily so tick() can read its info
                    cars.insert(car_id, car);
                    results.push((car_id, CarUpdateResult::Despawn));
                }
                Ok(CarUpdateResult::ArrivedAtDestination(dest)) => {
                    // Put car back temporarily so tick() can read its info
                    cars.insert(car_id, car);
                    results.push((car_id, CarUpdateResult::ArrivedAtDestination(dest)));
                }
                Err(_) => {
                    // Put car back temporarily so tick() can read its info
                    cars.insert(car_id, car);
                    results.push((car_id, CarUpdateResult::Despawn));
                }
            }
        }
    }

    results
}

/// Recalculate paths for all cars that might have invalid paths
///
/// This is called when roads are removed and cars need to find new routes
pub fn recalculate_car_paths(
    cars: &mut HashMap<CarId, SimCar>,
    road_network: &mut SimRoadNetwork,
    apartments: &mut HashMap<ApartmentId, SimApartment>,
    factories: &mut HashMap<FactoryId, SimFactory>,
) {
    let car_ids: Vec<CarId> = cars.keys().copied().collect();
    let mut cars_to_despawn = Vec::new();

    for car_id in car_ids {
        if let Some(car) = cars.get(&car_id) {
            // Get the car's final destination
            let destination = match car.path.last() {
                Some(dest) => *dest,
                None => continue, // No path to recalculate
            };

            // Get the current intersection the car is heading to
            let current_target = match car.path.first() {
                Some(target) => *target,
                None => continue,
            };

            // Try to find a new path from current target to destination
            let new_path = road_network.find_path(current_target, destination);

            match new_path {
                Some(path) => {
                    // Update the car's path
                    if let Some(car) = cars.get_mut(&car_id) {
                        car.path = std::iter::once(current_target).chain(path).collect();
                    }
                }
                None => {
                    // No valid path exists - mark for despawn
                    cars_to_despawn.push(car_id);
                }
            }
        }
    }

    // Despawn cars that can't find a path
    for car_id in cars_to_despawn {
        despawn_car(car_id, cars, road_network, apartments, factories);
    }
}
