//! Building types for the traffic simulation
//!
//! Houses, factories, and shops - standalone implementations.

use super::types::{CarId, FactoryId, HouseId, IntersectionId, ShopId};

/// Duration in seconds that a factory processes a car before sending it back
pub const FACTORY_PROCESSING_TIME: f32 = 2.0;
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
    /// Cars currently being processed (car_id, shop_target, time_remaining)
    pub processing_cars: Vec<(CarId, IntersectionId, f32)>,
    /// Current labor demand
    pub labor_demand: f32,
    /// Rate at which labor demand increases per second
    pub labor_demand_rate: f32,
    /// Current inventory of produced goods
    pub inventory: u32,
    /// Maximum inventory capacity
    pub max_inventory: u32,
}

impl SimFactory {
    pub fn new(id: FactoryId, intersection_id: IntersectionId) -> Self {
        Self {
            id,
            intersection_id,
            processing_cars: Vec::new(),
            labor_demand: 10.0,
            labor_demand_rate: 1.0,
            inventory: 0,
            max_inventory: 10,
        }
    }

    /// Receive a car (worker arrives) and start processing
    pub fn receive_car(&mut self, car_id: CarId, shop_target: IntersectionId) {
        self.labor_demand = (self.labor_demand - LABOR_DEMAND_PER_WORKER).max(0.0);
        self.processing_cars
            .push((car_id, shop_target, FACTORY_PROCESSING_TIME));
    }

    /// Try to reserve a worker slot. Returns true if accepted.
    pub fn try_reserve_worker(&mut self) -> bool {
        if self.labor_demand >= LABOR_DEMAND_THRESHOLD {
            self.labor_demand = (self.labor_demand - LABOR_DEMAND_PER_WORKER).max(0.0);
            true
        } else {
            false
        }
    }

    /// Update the factory logic
    /// Returns number of products produced this tick
    pub fn update(&mut self, delta_secs: f32) -> u32 {
        // Increase labor demand over time
        self.labor_demand += self.labor_demand_rate * delta_secs;

        // Update processing times
        let mut products_produced = 0;
        self.processing_cars
            .retain_mut(|(_car_id, _shop_target, time_remaining)| {
                *time_remaining -= delta_secs;
                if *time_remaining <= 0.0 {
                    products_produced += 1;
                    false
                } else {
                    true
                }
            });

        // Add produced goods to inventory
        if products_produced > 0 {
            let space_available = self.max_inventory.saturating_sub(self.inventory);
            let to_add = products_produced.min(space_available);
            self.inventory += to_add;
        }

        products_produced
    }

    /// Try to take one product from inventory for delivery
    pub fn take_product(&mut self) -> bool {
        if self.inventory > 0 {
            self.inventory -= 1;
            true
        } else {
            false
        }
    }
}

/// A shop in the simulation
#[derive(Debug, Clone)]
pub struct SimShop {
    pub id: ShopId,
    pub intersection_id: IntersectionId,
    /// Number of cars that have arrived
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
