//! Factory-specific logic for the traffic simulation
//!
//! This module contains all factory-related behavior including worker management
//! and truck dispatch decisions.

use super::building::SimFactory;
use super::types::HouseId;

/// Duration in seconds that a worker spends at the factory before returning home
pub const FACTORY_WORK_TIME: f32 = 5.0;
/// Threshold at which factories need workers
pub const LABOR_DEMAND_THRESHOLD: f32 = 10.0;
/// Amount of demand reduced when a worker arrives
pub const LABOR_DEMAND_PER_WORKER: f32 = 10.0;

impl SimFactory {
    /// Check if the factory can accept workers
    /// Workers can only be accepted when the truck is available (not out making deliveries)
    pub fn can_accept_workers(&self) -> bool {
        self.truck.is_none()
    }

    /// Receive a worker at the factory (store their house_id so we can send them home)
    /// Only accepts workers if truck is available (not out making deliveries)
    pub fn receive_worker(&mut self, house_id: HouseId) -> bool {
        if !self.can_accept_workers() {
            return false;
        }
        self.labor_demand = (self.labor_demand - LABOR_DEMAND_PER_WORKER).max(0.0);
        self.workers.push((house_id, FACTORY_WORK_TIME));
        true
    }

    /// Try to reserve a worker slot. Returns true if accepted.
    /// Only accepts workers if truck is available (not out making deliveries)
    pub fn try_reserve_worker(&mut self) -> bool {
        if !self.can_accept_workers() {
            return false;
        }
        if self.labor_demand >= LABOR_DEMAND_THRESHOLD {
            self.labor_demand = (self.labor_demand - LABOR_DEMAND_PER_WORKER).max(0.0);
            true
        } else {
            false
        }
    }

    /// Update the factory logic
    /// Returns list of house_ids for workers whose work is done (they should return home)
    pub fn update(&mut self, delta_secs: f32) -> Vec<HouseId> {
        // Increase labor demand over time
        self.labor_demand += self.labor_demand_rate * delta_secs;

        // Update worker times and find those done working
        let mut workers_done = Vec::new();
        self.workers.retain_mut(|(house_id, time_remaining)| {
            *time_remaining -= delta_secs;
            if *time_remaining <= 0.0 {
                workers_done.push(*house_id);
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
