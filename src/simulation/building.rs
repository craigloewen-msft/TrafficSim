//! Building types for the traffic simulation
//!
//! Houses, factories, and shops - standalone implementations.

use super::types::{CarId, FactoryId, HouseId, IntersectionId, ShopId};

/// Duration in seconds that a worker spends at the factory before returning home
pub const FACTORY_WORK_TIME: f32 = 5.0;
/// Threshold at which factories need workers
pub const LABOR_DEMAND_THRESHOLD: f32 = 10.0;
/// Amount of demand reduced when a worker arrives
pub const LABOR_DEMAND_PER_WORKER: f32 = 10.0;
/// Threshold at which shops want products
pub const PRODUCT_DEMAND_THRESHOLD: f32 = 10.0;
/// Amount of demand satisfied per product delivery
pub const PRODUCT_DEMAND_PER_DELIVERY: f32 = 10.0;

/// A house in the simulation
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SimHouse {
    pub id: HouseId,
    pub intersection_id: IntersectionId,
    /// The car owned by this house (if out driving)
    pub car: Option<CarId>,
}

impl SimHouse {
    pub fn new(id: HouseId, intersection_id: IntersectionId) -> Self {
        Self {
            id,
            intersection_id,
            car: None,
        }
    }
}

/// A factory in the simulation
#[derive(Debug, Clone)]
pub struct SimFactory {
    pub id: FactoryId,
    pub intersection_id: IntersectionId,
    /// Workers currently at the factory (house_id, time_remaining until work done)
    pub workers: Vec<(HouseId, f32)>,
    /// Current labor demand
    pub labor_demand: f32,
    /// Rate at which labor demand increases per second
    pub labor_demand_rate: f32,
    /// Number of deliveries ready to be sent (max 2)
    pub deliveries_ready: u32,
    /// Maximum number of deliveries that can be stored
    pub max_deliveries: u32,
    /// The truck owned by this factory (if out making delivery)
    pub truck: Option<CarId>,
}

impl SimFactory {
    pub fn new(id: FactoryId, intersection_id: IntersectionId) -> Self {
        Self {
            id,
            intersection_id,
            workers: Vec::new(),
            labor_demand: 10.0,
            labor_demand_rate: 1.0,
            deliveries_ready: 0,
            max_deliveries: 2,
            truck: None,
        }
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

    /// Check if the factory can accept workers
    /// Workers can only be accepted when:
    /// 1. The truck is available (not out making deliveries)
    /// 2. The factory is not full (has room for more deliveries)
    fn can_accept_workers(&self) -> bool {
        self.truck.is_none() && self.deliveries_ready < self.max_deliveries
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

/// A shop in the simulation
#[derive(Debug, Clone)]
pub struct SimShop {
    pub id: ShopId,
    pub intersection_id: IntersectionId,
    /// Number of deliveries received
    pub cars_received: usize,
    /// Current product demand
    pub product_demand: f32,
    /// Rate at which product demand increases per second
    pub product_demand_rate: f32,
}

impl SimShop {
    pub fn new(id: ShopId, intersection_id: IntersectionId) -> Self {
        Self {
            id,
            intersection_id,
            cars_received: 0,
            product_demand: 10.0,
            product_demand_rate: 1.0,
        }
    }

    /// Receive a delivery
    pub fn receive_delivery(&mut self) {
        self.cars_received += 1;
        self.product_demand = (self.product_demand - PRODUCT_DEMAND_PER_DELIVERY).max(0.0);
    }

    /// Update the shop logic
    pub fn update(&mut self, delta_secs: f32) {
        self.product_demand += self.product_demand_rate * delta_secs;
    }
}
