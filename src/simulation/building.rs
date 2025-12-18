//! Building types for the traffic simulation
//!
//! Apartments, factories, and shops - standalone implementations.

use super::types::{CarId, FactoryId, ApartmentId, IntersectionId, ShopId};

/// An apartment in the simulation
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SimApartment {
    pub id: ApartmentId,
    pub intersection_id: IntersectionId,
    /// The cars owned by this apartment (10 total, if out driving)
    pub cars: Vec<Option<CarId>>,
}

impl SimApartment {
    pub fn new(id: ApartmentId, intersection_id: IntersectionId) -> Self {
        Self {
            id,
            intersection_id,
            cars: vec![None; 10],
        }
    }
}

/// A factory in the simulation
#[derive(Debug, Clone)]
pub struct SimFactory {
    pub id: FactoryId,
    pub intersection_id: IntersectionId,
    /// Workers currently at the factory (apartment_id, time_remaining until work done)
    pub workers: Vec<(ApartmentId, f32)>,
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
