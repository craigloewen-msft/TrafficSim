//! Building types for the traffic simulation
//!
//! Houses, factories, and shops - standalone implementations.

use super::types::{CarId, FactoryId, HouseId, IntersectionId, ShopId};

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
}

impl SimShop {
    pub fn new(id: ShopId, intersection_id: IntersectionId) -> Self {
        Self {
            id,
            intersection_id,
            cars_received: 0,
        }
    }

    /// Receive a delivery
    pub fn receive_delivery(&mut self) {
        self.cars_received += 1;
    }
}
