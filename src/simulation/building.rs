//! Building types for the traffic simulation
//!
//! Houses, factories, and shops - standalone implementations.

use super::types::{CarId, FactoryId, HouseId, IntersectionId, ShopId};

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
