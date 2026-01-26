//! Stuck car detection and diagnostics for the traffic simulation
//!
//! Detects cars that have been waiting too long (stuck) and provides
//! detailed state dumps for debugging intersection issues.

use log::{error, warn};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::building::{SimApartment, SimFactory, SimShop};
use super::car::SimCar;
use super::intersection::SimIntersection;
use super::road_network::SimRoadNetwork;
use super::types::{CarId, IntersectionId};

/// Threshold in seconds before a car is considered stuck
pub const STUCK_THRESHOLD_SECS: f32 = 10.0;

/// Serializable snapshot of a car's state
#[derive(Debug, Serialize)]
pub struct CarSnapshot {
    pub car_id: String,
    pub position: PositionSnapshot,
    pub speed: f32,
    pub wait_time: f32,
    pub current_road: String,
    pub distance_along_road: f32,
    pub path: Vec<String>,
    pub vehicle_type: String,
    pub trip_type: String,
    pub origin_apartment: Option<String>,
    pub origin_factory: Option<String>,
}

/// Serializable position
#[derive(Debug, Serialize)]
pub struct PositionSnapshot {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Serializable snapshot of an intersection's state
#[derive(Debug, Serialize)]
pub struct IntersectionSnapshot {
    pub intersection_id: String,
    pub position: PositionSnapshot,
    pub occupied_by: Option<String>,
    pub occupation_timer: f32,
    pub crossing_time: f32,
}

/// Serializable snapshot of a road's state
#[derive(Debug, Serialize)]
pub struct RoadSnapshot {
    pub road_id: String,
    pub start_intersection: String,
    pub end_intersection: String,
    pub length: f32,
    pub cars_on_road: Vec<String>,
}

/// Complete state dump when a stuck car is detected
#[derive(Debug, Serialize)]
pub struct StuckCarDump {
    pub simulation_time: f32,
    pub stuck_car: CarSnapshot,
    pub target_intersection: Option<IntersectionSnapshot>,
    pub nearby_intersections: Vec<IntersectionSnapshot>,
    pub nearby_cars: Vec<CarSnapshot>,
    pub current_road: Option<RoadSnapshot>,
    pub connected_roads: Vec<RoadSnapshot>,
}

impl CarSnapshot {
    pub fn from_car(car: &SimCar) -> Self {
        Self {
            car_id: format!("{:?}", car.id.0),
            position: PositionSnapshot {
                x: car.position.x,
                y: car.position.y,
                z: car.position.z,
            },
            speed: car.speed,
            wait_time: car.wait_time,
            current_road: format!("{:?}", car.current_road.0),
            distance_along_road: car.distance_along_road.into_inner(),
            path: car.path.iter().map(|id| format!("{:?}", id.0)).collect(),
            vehicle_type: format!("{:?}", car.vehicle_type),
            trip_type: format!("{:?}", car.trip_type),
            origin_apartment: car.origin_apartment.map(|id| format!("{:?}", id.0)),
            origin_factory: car.origin_factory.map(|id| format!("{:?}", id.0)),
        }
    }
}

impl IntersectionSnapshot {
    pub fn from_intersection(intersection: &SimIntersection) -> Self {
        Self {
            intersection_id: format!("{:?}", intersection.id.0),
            position: PositionSnapshot {
                x: intersection.position.x,
                y: intersection.position.y,
                z: intersection.position.z,
            },
            occupied_by: intersection.occupied_by.map(|id| format!("{:?}", id.0)),
            occupation_timer: intersection.occupation_timer,
            crossing_time: intersection.crossing_time,
        }
    }
}

/// Find all cars that are stuck (waiting longer than threshold)
pub fn find_stuck_cars(cars: &HashMap<CarId, SimCar>) -> Vec<CarId> {
    cars.iter()
        .filter(|(_, car)| car.wait_time >= STUCK_THRESHOLD_SECS)
        .map(|(id, _)| *id)
        .collect()
}

/// Generate a detailed state dump for a stuck car
pub fn generate_stuck_car_dump(
    stuck_car_id: CarId,
    simulation_time: f32,
    cars: &HashMap<CarId, SimCar>,
    intersections: &HashMap<IntersectionId, SimIntersection>,
    road_network: &SimRoadNetwork,
    _apartments: &HashMap<super::types::ApartmentId, SimApartment>,
    _factories: &HashMap<super::types::FactoryId, SimFactory>,
    _shops: &HashMap<super::types::ShopId, SimShop>,
) -> Option<StuckCarDump> {
    let stuck_car = cars.get(&stuck_car_id)?;

    // Get the target intersection (first in path)
    let target_intersection_id = stuck_car.path.first();
    let target_intersection = target_intersection_id
        .and_then(|id| intersections.get(id))
        .map(IntersectionSnapshot::from_intersection);

    // Get the current road info
    let current_road = road_network.get_road(stuck_car.current_road);
    let current_road_snapshot = current_road.map(|road| {
        let cars_on_road = road_network
            .get_cars_on_road(stuck_car.current_road)
            .iter()
            .map(|id| format!("{:?}", id.0))
            .collect();
        RoadSnapshot {
            road_id: format!("{:?}", road.id.0),
            start_intersection: format!("{:?}", road.start_intersection.0),
            end_intersection: format!("{:?}", road.end_intersection.0),
            length: road.length,
            cars_on_road,
        }
    });

    // Find nearby cars (within 5 units)
    let nearby_cars: Vec<CarSnapshot> = cars
        .values()
        .filter(|car| {
            car.id != stuck_car_id && stuck_car.position.distance(&car.position) <= 5.0
        })
        .map(CarSnapshot::from_car)
        .collect();

    // Find nearby intersections (within 10 units)
    let nearby_intersections: Vec<IntersectionSnapshot> = intersections
        .values()
        .filter(|int| stuck_car.position.distance(&int.position) <= 10.0)
        .map(IntersectionSnapshot::from_intersection)
        .collect();

    // Find connected roads from nearby intersections
    let mut connected_roads = Vec::new();
    if let Some(target_id) = target_intersection_id {
        for road in road_network.roads().values() {
            if road.start_intersection == *target_id || road.end_intersection == *target_id {
                let cars_on_road = road_network
                    .get_cars_on_road(road.id)
                    .iter()
                    .map(|id| format!("{:?}", id.0))
                    .collect();
                connected_roads.push(RoadSnapshot {
                    road_id: format!("{:?}", road.id.0),
                    start_intersection: format!("{:?}", road.start_intersection.0),
                    end_intersection: format!("{:?}", road.end_intersection.0),
                    length: road.length,
                    cars_on_road,
                });
            }
        }
    }

    Some(StuckCarDump {
        simulation_time,
        stuck_car: CarSnapshot::from_car(stuck_car),
        target_intersection,
        nearby_intersections,
        nearby_cars,
        current_road: current_road_snapshot,
        connected_roads,
    })
}

/// Output the stuck car dump to console
pub fn dump_to_console(dump: &StuckCarDump) {
    error!("========================================");
    error!("STUCK CAR DETECTED - STATE DUMP");
    error!("========================================");
    error!("Simulation Time: {:.2}s", dump.simulation_time);
    error!("");
    error!("--- STUCK CAR ---");
    error!("  ID: {}", dump.stuck_car.car_id);
    error!(
        "  Position: ({:.2}, {:.2}, {:.2})",
        dump.stuck_car.position.x, dump.stuck_car.position.y, dump.stuck_car.position.z
    );
    error!("  Speed: {:.2}", dump.stuck_car.speed);
    error!("  Wait Time: {:.2}s", dump.stuck_car.wait_time);
    error!("  Current Road: {}", dump.stuck_car.current_road);
    error!(
        "  Distance Along Road: {:.2}",
        dump.stuck_car.distance_along_road
    );
    error!("  Path: {:?}", dump.stuck_car.path);
    error!("  Vehicle Type: {}", dump.stuck_car.vehicle_type);
    error!("  Trip Type: {}", dump.stuck_car.trip_type);

    if let Some(ref target) = dump.target_intersection {
        error!("");
        error!("--- TARGET INTERSECTION ---");
        error!("  ID: {}", target.intersection_id);
        error!(
            "  Position: ({:.2}, {:.2}, {:.2})",
            target.position.x, target.position.y, target.position.z
        );
        error!(
            "  Occupied By: {}",
            target.occupied_by.as_deref().unwrap_or("None")
        );
        error!("  Occupation Timer: {:.2}s", target.occupation_timer);
        error!("  Crossing Time: {:.2}s", target.crossing_time);
    }

    if !dump.nearby_cars.is_empty() {
        error!("");
        error!("--- NEARBY CARS ({}) ---", dump.nearby_cars.len());
        for car in &dump.nearby_cars {
            error!(
                "  {} at ({:.2}, {:.2}, {:.2}) - wait: {:.2}s, road: {}",
                car.car_id, car.position.x, car.position.y, car.position.z, car.wait_time, car.current_road
            );
        }
    }

    if !dump.nearby_intersections.is_empty() {
        error!("");
        error!(
            "--- NEARBY INTERSECTIONS ({}) ---",
            dump.nearby_intersections.len()
        );
        for int in &dump.nearby_intersections {
            error!(
                "  {} - occupied by: {}, timer: {:.2}s",
                int.intersection_id,
                int.occupied_by.as_deref().unwrap_or("None"),
                int.occupation_timer
            );
        }
    }

    if !dump.connected_roads.is_empty() {
        error!("");
        error!(
            "--- CONNECTED ROADS ({}) ---",
            dump.connected_roads.len()
        );
        for road in &dump.connected_roads {
            error!(
                "  {} ({} -> {}) - {} cars: {:?}",
                road.road_id,
                road.start_intersection,
                road.end_intersection,
                road.cars_on_road.len(),
                road.cars_on_road
            );
        }
    }

    error!("========================================");
    warn!("Stuck car {} will be despawned for recovery", dump.stuck_car.car_id);
    error!("========================================");
}

/// Output the stuck car dump to a JSON file
pub fn dump_to_json(dump: &StuckCarDump, output_dir: &Path) -> std::io::Result<String> {
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;

    // Generate filename with timestamp
    let filename = format!(
        "stuck_car_dump_{}_t{:.0}.json",
        dump.stuck_car.car_id.replace(['(', ')', ' '], ""),
        dump.simulation_time
    );
    let filepath = output_dir.join(&filename);

    // Serialize to JSON
    let json = serde_json::to_string_pretty(dump)
        .map_err(std::io::Error::other)?;

    // Write to file
    fs::write(&filepath, &json)?;

    Ok(filepath.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::types::{SimId, TripType, VehicleType, Position};
    use ordered_float::OrderedFloat;

    fn create_test_car(id: usize, wait_time: f32) -> SimCar {
        SimCar {
            id: CarId(SimId(id)),
            speed: 5.0,
            current_road: super::super::types::RoadId(SimId(100)),
            distance_along_road: OrderedFloat(0.0),
            start_intersection: IntersectionId(SimId(1)),
            path: vec![IntersectionId(SimId(2))],
            position: Position::new(0.0, 0.0, 0.0),
            angle: 0.0,
            vehicle_type: VehicleType::Car,
            trip_type: TripType::Outbound,
            origin_apartment: None,
            origin_factory: None,
            wait_time,
        }
    }

    #[test]
    fn test_find_stuck_cars_none() {
        let mut cars = HashMap::new();
        cars.insert(CarId(SimId(1)), create_test_car(1, 0.0));
        cars.insert(CarId(SimId(2)), create_test_car(2, 5.0));
        cars.insert(CarId(SimId(3)), create_test_car(3, 9.9));

        let stuck = find_stuck_cars(&cars);
        assert!(stuck.is_empty());
    }

    #[test]
    fn test_find_stuck_cars_one() {
        let mut cars = HashMap::new();
        cars.insert(CarId(SimId(1)), create_test_car(1, 0.0));
        cars.insert(CarId(SimId(2)), create_test_car(2, 10.0)); // Exactly at threshold
        cars.insert(CarId(SimId(3)), create_test_car(3, 5.0));

        let stuck = find_stuck_cars(&cars);
        assert_eq!(stuck.len(), 1);
        assert_eq!(stuck[0], CarId(SimId(2)));
    }

    #[test]
    fn test_find_stuck_cars_multiple() {
        let mut cars = HashMap::new();
        cars.insert(CarId(SimId(1)), create_test_car(1, 15.0)); // Stuck
        cars.insert(CarId(SimId(2)), create_test_car(2, 10.0)); // Stuck
        cars.insert(CarId(SimId(3)), create_test_car(3, 5.0));  // Not stuck

        let stuck = find_stuck_cars(&cars);
        assert_eq!(stuck.len(), 2);
    }

    #[test]
    fn test_car_snapshot_from_car() {
        let car = create_test_car(42, 12.5);
        let snapshot = CarSnapshot::from_car(&car);

        assert_eq!(snapshot.wait_time, 12.5);
        assert_eq!(snapshot.speed, 5.0);
        assert_eq!(snapshot.vehicle_type, "Car");
        assert_eq!(snapshot.trip_type, "Outbound");
    }
}

/// Process stuck cars: generate dumps and return list of cars to despawn
pub fn process_stuck_cars(
    simulation_time: f32,
    cars: &HashMap<CarId, SimCar>,
    intersections: &HashMap<IntersectionId, SimIntersection>,
    road_network: &SimRoadNetwork,
    apartments: &HashMap<super::types::ApartmentId, SimApartment>,
    factories: &HashMap<super::types::FactoryId, SimFactory>,
    shops: &HashMap<super::types::ShopId, SimShop>,
    output_dir: &Path,
) -> Vec<CarId> {
    let stuck_cars = find_stuck_cars(cars);

    for car_id in &stuck_cars {
        if let Some(dump) = generate_stuck_car_dump(
            *car_id,
            simulation_time,
            cars,
            intersections,
            road_network,
            apartments,
            factories,
            shops,
        ) {
            // Output to console
            dump_to_console(&dump);

            // Output to JSON file
            match dump_to_json(&dump, output_dir) {
                Ok(filepath) => {
                    warn!("Stuck car dump written to: {}", filepath);
                }
                Err(e) => {
                    error!("Failed to write stuck car dump to file: {}", e);
                }
            }
        }
    }

    stuck_cars
}
