//! Car movement logic for the traffic simulation
//!
//! Standalone implementation that doesn't depend on Bevy.

use anyhow::{Context, Result};
use ordered_float::OrderedFloat;

use super::intersection::SimIntersection;
use super::road_network::SimRoadNetwork;
use super::types::{
    CarId, FactoryId, ApartmentId, IntersectionId, Position, RoadId, TripType, VehicleType, CAR_LENGTH,
    INTERSECTION_APPROACH_DISTANCE, SAFE_FOLLOWING_MULTIPLIER,
};

/// Result of a car update indicating what action should be taken
#[derive(Debug, Clone)]
pub enum CarUpdateResult {
    Continue,                             // Car continues moving
    Despawn,                              // Car should be despawned
    ArrivedAtDestination(IntersectionId), // Car arrived at destination
}

/// A car in the traffic simulation
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SimCar {
    pub id: CarId,
    pub speed: f32,
    pub current_road: RoadId,
    pub distance_along_road: OrderedFloat<f32>,
    pub start_intersection: IntersectionId,
    pub path: Vec<IntersectionId>,
    pub position: Position,
    pub angle: f32,
    /// Type of vehicle (Car or Truck)
    pub vehicle_type: VehicleType,
    /// Type of trip (Outbound to destination, or Return to origin)
    pub trip_type: TripType,
    /// The apartment this car belongs to (for cars)
    pub origin_apartment: Option<ApartmentId>,
    /// The factory this truck belongs to (for trucks)
    pub origin_factory: Option<FactoryId>,
}

impl SimCar {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: CarId,
        speed: f32,
        current_road: RoadId,
        start_intersection: IntersectionId,
        path: Vec<IntersectionId>,
        position: Position,
        angle: f32,
        vehicle_type: VehicleType,
        trip_type: TripType,
        origin_apartment: Option<ApartmentId>,
        origin_factory: Option<FactoryId>,
    ) -> Self {
        Self {
            id,
            speed,
            current_road,
            distance_along_road: OrderedFloat(0.0),
            start_intersection,
            path,
            position,
            angle,
            vehicle_type,
            trip_type,
            origin_apartment,
            origin_factory,
        }
    }

    /// Update car movement logic
    /// Returns CarUpdateResult indicating what action should be taken with the car
    pub fn update(
        &mut self,
        delta_secs: f32,
        road_network: &mut SimRoadNetwork,
        intersections: &mut std::collections::HashMap<IntersectionId, SimIntersection>,
    ) -> Result<CarUpdateResult> {
        // Check if we've reached the final destination
        if self.path.is_empty() {
            return Ok(CarUpdateResult::Despawn);
        }

        // Get the current road
        let current_road = road_network
            .get_road(self.current_road)
            .context("Road not found")?
            .clone();

        // Get the target intersection (first item in path)
        let target_intersection_id = *self.path.first().context("Path is empty")?;

        // Get start and end intersection positions
        let start_pos = *road_network
            .get_intersection_position(self.start_intersection)
            .context("Start intersection not found")?;

        let end_pos = *road_network
            .get_intersection_position(target_intersection_id)
            .context("Target intersection not found")?;

        let road_length = current_road.length;

        let prev_road = self.current_road;
        let prev_distance = self.distance_along_road;

        // Check for car ahead
        let ahead_car_option = road_network
            .find_car_ahead_on_road(self.current_road, &self.distance_along_road)
            .ok()
            .flatten();

        // Update distance along the road
        let mut distance_delta = self.speed * delta_secs;

        // Track whether we're blocked by a car ahead
        let mut blocked_by_car_ahead = false;

        if let Some((ahead_distance, _)) = ahead_car_option {
            let ahead_car_distance_diff = ahead_distance - self.distance_along_road;
            let safe_following_distance = CAR_LENGTH * SAFE_FOLLOWING_MULTIPLIER;
            if ahead_car_distance_diff <= OrderedFloat(distance_delta + safe_following_distance) {
                distance_delta = 0.0;
                blocked_by_car_ahead = true;
            }
        }

        // Check if we're approaching the end of the road
        // Only try to acquire intersection lock if we're not blocked by a car ahead
        // BUT if we already hold the lock, we still need to check if we can proceed
        // This prevents acquiring new locks when blocked, while maintaining existing locks
        let distance_to_intersection = road_length - self.distance_along_road.into_inner();

        if distance_to_intersection <= INTERSECTION_APPROACH_DISTANCE {
            let target_intersection = intersections
                .get_mut(&target_intersection_id)
                .context("Failed to get intersection")?;

            // Only check/acquire intersection if:
            // 1. We're not blocked by a car ahead, OR
            // 2. We already hold the lock on this intersection
            if (!blocked_by_car_ahead || target_intersection.is_held_by(self.id))
                && !target_intersection.can_proceed(self.id)
            {
                distance_delta = 0.0;
            }
        }

        self.distance_along_road += distance_delta;

        // Check if we've reached the end of the current road
        if self.distance_along_road >= OrderedFloat(road_length) {
            // Remove the intersection we just reached from the path
            let reached_intersection = self.path.remove(0);

            // Release the intersection lock
            if let Some(intersection) = intersections.get_mut(&reached_intersection) {
                intersection.release(self.id);
            }

            if self.path.is_empty() {
                self.distance_along_road = OrderedFloat(road_length);
                self.position = end_pos;

                road_network.update_car_road_position(
                    self.id,
                    self.current_road,
                    self.distance_along_road,
                    true,
                    Some(prev_road),
                    prev_distance,
                )?;

                return Ok(CarUpdateResult::ArrivedAtDestination(reached_intersection));
            }

            let next_intersection_id = *self.path.first().context("No next intersection")?;

            // Find the road connecting to next intersection
            let next_road_id = road_network
                .find_road_between(reached_intersection, next_intersection_id)
                .context("No road found between intersections")?;

            self.current_road = next_road_id;
            self.distance_along_road = OrderedFloat(0.0);

            let new_road = road_network
                .get_road(next_road_id)
                .context("Failed to get next road")?;

            self.start_intersection = new_road.start_intersection;
            self.angle = new_road.angle;
        } else {
            // Interpolate position along current road
            let progress_ratio = self.distance_along_road.into_inner() / road_length;
            let mut position = start_pos.lerp(&end_pos, progress_ratio);

            // Apply lane offset for two-way roads
            if current_road.is_two_way {
                const LANE_OFFSET: f32 = 0.15;
                let offset = start_pos.perpendicular_offset(&end_pos, LANE_OFFSET);
                position.x += offset.x;
                position.z += offset.z;
            }

            self.position = position;
        }

        road_network.update_car_road_position(
            self.id,
            self.current_road,
            self.distance_along_road,
            false,
            Some(prev_road),
            prev_distance,
        )?;

        Ok(CarUpdateResult::Continue)
    }
}
