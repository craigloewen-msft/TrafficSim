//! Factory-specific logic for the traffic simulation
//!
//! This module contains all factory-related behavior including worker management
//! and truck dispatch decisions.

use super::building::SimFactory;
use super::types::ApartmentId;

/// Duration in seconds that a worker spends at the factory before returning home
pub const FACTORY_WORK_TIME: f32 = 5.0;

impl SimFactory {
    /// Check if the factory can accept workers
    /// Workers can only be accepted when the truck is available (not out making deliveries)
    pub fn can_accept_workers(&self) -> bool {
        self.truck.is_none()
    }

    /// Receive a worker at the factory (store their apartment_id so we can send them home)
    /// Only accepts workers if truck is available (not out making deliveries)
    pub fn receive_worker(&mut self, apartment_id: ApartmentId) -> bool {
        if !self.can_accept_workers() {
            return false;
        }
        self.workers.push((apartment_id, FACTORY_WORK_TIME));
        true
    }

    /// Update the factory logic
    /// Returns list of apartment_ids for workers whose work is done (they should return home)
    pub fn update(&mut self, delta_secs: f32) -> Vec<ApartmentId> {
        // Update worker times and find those done working
        let mut workers_done = Vec::new();
        self.workers.retain_mut(|(apartment_id, time_remaining)| {
            *time_remaining -= delta_secs;
            if *time_remaining <= 0.0 {
                workers_done.push(*apartment_id);
                // Add to deliveries when worker finishes
                if self.deliveries_ready < self.max_deliveries {
                    self.deliveries_ready += 1;
                }
                false
            } else {
                true
            }
        });

        workers_done
    }

    /// Try to take one delivery for truck dispatch
    /// Note: This check also verifies truck is home for safety, though callers should ensure this
    pub fn take_delivery(&mut self) -> bool {
        if self.deliveries_ready > 0 && self.truck.is_none() {
            self.deliveries_ready -= 1;
            true
        } else {
            false
        }
    }

    /// Check if the factory's truck is available
    pub fn truck_available(&self) -> bool {
        self.truck.is_none()
    }
}
