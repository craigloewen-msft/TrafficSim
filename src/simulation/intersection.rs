//! Intersection logic for the traffic simulation
//! 
//! Standalone implementation that doesn't depend on Bevy.

use super::types::{CarId, IntersectionId, Position};

/// An intersection in the traffic simulation
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SimIntersection {
    pub id: IntersectionId,
    pub position: Position,
    /// The car currently occupying the intersection (if any)
    pub occupied_by: Option<CarId>,
    /// Timer for how long the current car has been in the intersection
    pub occupation_timer: f32,
    /// Time it takes for a car to cross through the intersection
    pub crossing_time: f32,
}

impl SimIntersection {
    pub fn new(id: IntersectionId, position: Position) -> Self {
        Self {
            id,
            position,
            occupied_by: None,
            occupation_timer: 0.0,
            crossing_time: 0.25,
        }
    }

    /// Release the intersection lock
    pub fn release(&mut self, car_id: CarId) {
        if let Some(current_car) = self.occupied_by {
            if current_car == car_id {
                self.occupied_by = None;
                self.occupation_timer = 0.0;
            }
        }
    }

    /// Check if a car can proceed through the intersection
    /// This handles both acquiring the lock and checking wait time
    /// Returns true if the car can proceed, false if it must wait
    pub fn can_proceed(&mut self, car_id: CarId) -> bool {
        match self.occupied_by {
            None => {
                // Intersection is free, acquire it and start crossing
                self.occupied_by = Some(car_id);
                self.occupation_timer = 0.0;
                false // Must wait the crossing time
            }
            Some(current_car) if current_car == car_id => {
                // This car already has the lock, check if crossing time has elapsed
                self.occupation_timer >= self.crossing_time
            }
            Some(_) => {
                // Another car has the lock, must wait
                false
            }
        }
    }

    /// Update the occupation timer
    pub fn update_timer(&mut self, delta_time: f32) {
        if self.occupied_by.is_some() {
            self.occupation_timer += delta_time;
        }
    }
}
