//! Trip orchestration logic for the traffic simulation
//!
//! This module handles all trip-related decisions:
//! - Determining which workers should spawn from apartments
//! - Processing vehicle arrivals at destinations
//! - Handling vehicle despawns and cleanup

use log::warn;
use std::collections::HashMap;

use super::building::{SimApartment, SimFactory, SimShop};
use super::game_state::GameState;
use super::road_network::SimRoadNetwork;
use super::types::{ApartmentId, CarId, FactoryId, IntersectionId, TripType, VehicleType};

/// Request to spawn a worker car from an apartment
#[derive(Debug, Clone)]
pub struct WorkerSpawnRequest {
    pub apartment_id: ApartmentId,
    pub apartment_intersection: IntersectionId,
    pub factory_intersection: IntersectionId,
    pub slot_index: usize,
}

/// Request to dispatch a truck from a factory
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TruckDispatchRequest {
    pub factory_id: FactoryId,
    pub factory_intersection: IntersectionId,
    pub shop_intersection: IntersectionId,
}

/// Request to send a worker home from a factory
#[derive(Debug, Clone)]
pub struct WorkerReturnRequest {
    pub factory_id: FactoryId,
    pub factory_intersection: IntersectionId,
    pub apartment_id: ApartmentId,
    pub apartment_intersection: IntersectionId,
}

/// Result of a worker arriving at a factory
#[derive(Debug)]
#[allow(dead_code)]
pub enum WorkerArrivalResult {
    /// Worker was accepted at the factory
    Accepted { destination_factory: FactoryId },
    /// Worker was rejected - needs to return home
    Rejected {
        destination_factory: Option<FactoryId>,
        apartment_intersection: IntersectionId,
    },
}

/// Result of handling an arrival
#[derive(Debug)]
#[allow(dead_code)]
pub enum ArrivalAction {
    /// Remove car from tracking, no follow-up spawn needed
    RemoveCar,
    /// Spawn a return vehicle (worker going home or truck returning)
    SpawnReturn {
        from_intersection: IntersectionId,
        to_intersection: IntersectionId,
        vehicle_type: VehicleType,
        origin_apartment: Option<ApartmentId>,
        origin_factory: Option<FactoryId>,
    },
}

/// Determine which workers should spawn from apartments to factories
///
/// Returns a list of spawn requests. The caller is responsible for actually
/// spawning the vehicles and updating apartment slots.
pub fn determine_workers_to_spawn<F>(
    apartments: &HashMap<ApartmentId, SimApartment>,
    factories: &HashMap<FactoryId, SimFactory>,
    mut choose_random: F,
) -> Vec<WorkerSpawnRequest>
where
    F: FnMut(&[(FactoryId, IntersectionId)]) -> Option<(FactoryId, IntersectionId)>,
{
    // Get all factories that can accept workers (truck is home)
    let factories_accepting: Vec<(FactoryId, IntersectionId)> = factories
        .values()
        .filter(|f| f.can_accept_workers())
        .map(|f| (f.id, f.intersection_id))
        .collect();

    if factories_accepting.is_empty() {
        return Vec::new();
    }

    let mut spawn_requests = Vec::new();

    for (apartment_id, apartment) in apartments {
        let apartment_intersection = apartment.intersection_id;

        // Find the first empty slot - only spawn ONE car per apartment per tick
        for (slot_index, car_slot) in apartment.cars.iter().enumerate() {
            if car_slot.is_none() {
                // Choose random factory
                if let Some((_factory_id, factory_intersection)) =
                    choose_random(&factories_accepting)
                {
                    spawn_requests.push(WorkerSpawnRequest {
                        apartment_id: *apartment_id,
                        apartment_intersection,
                        factory_intersection,
                        slot_index,
                    });
                }
                break; // Only spawn one car per apartment per tick
            }
        }
    }

    spawn_requests
}

/// Process workers done at factories and generate return requests
pub fn process_workers_done(
    workers_done: &[(FactoryId, ApartmentId)],
    factories: &HashMap<FactoryId, SimFactory>,
    apartments: &HashMap<ApartmentId, SimApartment>,
) -> Vec<WorkerReturnRequest> {
    let mut requests = Vec::new();

    for (factory_id, apartment_id) in workers_done {
        let apartment_intersection = match apartments.get(apartment_id) {
            Some(a) => a.intersection_id,
            None => continue,
        };

        let factory_intersection = match factories.get(factory_id) {
            Some(f) => f.intersection_id,
            None => continue,
        };

        requests.push(WorkerReturnRequest {
            factory_id: *factory_id,
            factory_intersection,
            apartment_id: *apartment_id,
            apartment_intersection,
        });
    }

    requests
}

/// Handle a worker arriving at a factory (outbound trip complete)
///
/// Returns whether the worker was accepted and any necessary follow-up action.
pub fn handle_worker_arrival_at_factory(
    car_id: CarId,
    destination: IntersectionId,
    origin_apartment: Option<ApartmentId>,
    factories: &mut HashMap<FactoryId, SimFactory>,
    apartments: &mut HashMap<ApartmentId, SimApartment>,
) -> (WorkerArrivalResult, bool) {
    let mut worker_accepted = false;
    let mut destination_factory: Option<FactoryId> = None;

    if let Some(apartment_id) = origin_apartment {
        if let Some((factory_id, factory)) = factories
            .iter_mut()
            .find(|(_, f)| f.intersection_id == destination)
        {
            worker_accepted = factory.receive_worker(apartment_id);
            destination_factory = Some(*factory_id);
        }
    }

    if worker_accepted {
        // Clear apartment slot since worker is at factory
        if let Some(apartment_id) = origin_apartment {
            if let Some(apartment) = apartments.get_mut(&apartment_id) {
                apartment.clear_car_slot(car_id);
            }
        }
        (
            WorkerArrivalResult::Accepted {
                destination_factory: destination_factory.unwrap(),
            },
            true,
        )
    } else {
        // Factory rejected worker - need to send back home
        let apartment_intersection = origin_apartment
            .and_then(|id| apartments.get(&id))
            .map(|a| a.intersection_id);

        if let Some(apt_intersection) = apartment_intersection {
            (
                WorkerArrivalResult::Rejected {
                    destination_factory,
                    apartment_intersection: apt_intersection,
                },
                false,
            )
        } else {
            // No apartment found, just remove
            (
                WorkerArrivalResult::Accepted {
                    destination_factory: destination_factory.unwrap_or(FactoryId(
                        super::types::SimId(0),
                    )),
                },
                true,
            )
        }
    }
}

/// Handle worker return car spawn result - update apartment slot
pub fn handle_worker_return_spawn_result(
    old_car_id: CarId,
    new_car_id: Option<CarId>,
    apartment_id: ApartmentId,
    apartments: &mut HashMap<ApartmentId, SimApartment>,
) {
    if let Some(apartment) = apartments.get_mut(&apartment_id) {
        match new_car_id {
            Some(new_id) => {
                // Update slot with new car ID
                for car_slot in &mut apartment.cars {
                    if *car_slot == Some(old_car_id) {
                        *car_slot = Some(new_id);
                        break;
                    }
                }
            }
            None => {
                // Failed to spawn, clear slot
                apartment.clear_car_slot(old_car_id);
            }
        }
    }
}

/// Calculate commute distance for a returning worker
pub fn calculate_commute_distance(
    origin_apartment: Option<ApartmentId>,
    origin_factory: Option<FactoryId>,
    apartments: &HashMap<ApartmentId, SimApartment>,
    factories: &HashMap<FactoryId, SimFactory>,
    road_network: &SimRoadNetwork,
) -> f32 {
    match (origin_apartment, origin_factory) {
        (Some(apartment_id), Some(factory_id)) => {
            let apartment_position = apartments.get(&apartment_id).and_then(|apartment| {
                road_network.get_intersection_position(apartment.intersection_id)
            });
            let factory_position = factories.get(&factory_id).and_then(|factory| {
                road_network.get_intersection_position(factory.intersection_id)
            });

            match (apartment_position, factory_position) {
                (Some(apartment_pos), Some(factory_pos)) => apartment_pos.distance(factory_pos),
                _ => {
                    warn!(
                        "Missing apartment or factory position for worker commute; defaulting to a zero-distance commute, which applies the maximum commute penalty"
                    );
                    0.0
                }
            }
        }
        _ => {
            warn!(
                "Missing worker identifiers for commute penalty; defaulting to a zero-distance commute, which applies the maximum commute penalty"
            );
            0.0
        }
    }
}

/// Handle a worker returning home (return trip complete)
///
/// Returns the commute distance for game state tracking
pub fn handle_worker_return_home(
    car_id: CarId,
    origin_apartment: Option<ApartmentId>,
    origin_factory: Option<FactoryId>,
    apartments: &mut HashMap<ApartmentId, SimApartment>,
    factories: &HashMap<FactoryId, SimFactory>,
    road_network: &SimRoadNetwork,
    game_state: &mut Option<GameState>,
) -> f32 {
    let commute_distance = calculate_commute_distance(
        origin_apartment,
        origin_factory,
        apartments,
        factories,
        road_network,
    );

    // Clear car slot
    if let Some(apartment_id) = origin_apartment {
        if let Some(apartment) = apartments.get_mut(&apartment_id) {
            apartment.clear_car_slot(car_id);
        }
    }

    // Track completion in game state
    if let Some(gs) = game_state {
        gs.complete_worker_trip(commute_distance);
    }

    commute_distance
}

/// Handle truck delivery arrival at shop
///
/// Returns the factory intersection for spawning return truck
pub fn handle_truck_delivery(
    destination: IntersectionId,
    origin_factory: Option<FactoryId>,
    shops: &mut HashMap<super::types::ShopId, SimShop>,
    factories: &HashMap<FactoryId, SimFactory>,
) -> Option<IntersectionId> {
    // Deliver to shop
    if let Some(shop) = shops.values_mut().find(|s| s.intersection_id == destination) {
        shop.receive_delivery();
    }

    // Get factory intersection for return trip
    origin_factory.and_then(|fid| factories.get(&fid).map(|f| f.intersection_id))
}

/// Handle truck returning to factory
pub fn handle_truck_return(
    origin_factory: Option<FactoryId>,
    factories: &mut HashMap<FactoryId, SimFactory>,
    game_state: &mut Option<GameState>,
) {
    // Clear truck reference
    if let Some(factory_id) = origin_factory {
        if let Some(factory) = factories.get_mut(&factory_id) {
            factory.truck = None;
        }
    }

    // Track completion
    if let Some(gs) = game_state {
        gs.complete_shop_delivery();
    }
}

/// Handle truck return spawn result - update factory truck reference
pub fn handle_truck_return_spawn_result(
    factory_id: FactoryId,
    new_truck_id: Option<CarId>,
    factories: &mut HashMap<FactoryId, SimFactory>,
) {
    if let Some(factory) = factories.get_mut(&factory_id) {
        factory.truck = new_truck_id;
    }
}

/// Handle vehicle despawn (cleanup references)
pub fn handle_vehicle_despawn(
    car_id: CarId,
    origin_apartment: Option<ApartmentId>,
    origin_factory: Option<FactoryId>,
    vehicle_type: VehicleType,
    apartments: &mut HashMap<ApartmentId, SimApartment>,
    factories: &mut HashMap<FactoryId, SimFactory>,
) {
    // Clean up apartment reference for cars
    if let Some(apartment_id) = origin_apartment {
        if let Some(apartment) = apartments.get_mut(&apartment_id) {
            apartment.clear_car_slot(car_id);
        }
    }

    // Clean up factory truck reference for trucks
    if vehicle_type == VehicleType::Truck {
        if let Some(factory_id) = origin_factory {
            if let Some(factory) = factories.get_mut(&factory_id) {
                if factory.truck == Some(car_id) {
                    factory.truck = None;
                }
            }
        }
    }
}

/// Information extracted from a car for processing arrivals
#[derive(Debug, Clone)]
pub struct CarInfo {
    pub vehicle_type: VehicleType,
    pub trip_type: TripType,
    pub origin_apartment: Option<ApartmentId>,
    pub origin_factory: Option<FactoryId>,
}

impl CarInfo {
    pub fn from_car(car: &super::car::SimCar) -> Self {
        Self {
            vehicle_type: car.vehicle_type,
            trip_type: car.trip_type,
            origin_apartment: car.origin_apartment,
            origin_factory: car.origin_factory,
        }
    }
}
