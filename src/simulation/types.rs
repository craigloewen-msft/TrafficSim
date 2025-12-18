//! Core types for the traffic simulation
//!
//! These are standalone types that don't depend on Bevy.

/// A unique identifier for simulation entities
/// This is a simple wrapper around a usize for type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SimId(pub usize);

/// Type of vehicle in the simulation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleType {
    /// Regular car from a house
    Car,
    /// Delivery truck from a factory
    Truck,
}

/// The type of trip a vehicle is making
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TripType {
    /// Going to destination (work for cars, delivery for trucks)
    Outbound,
    /// Returning to origin (home for cars, factory for trucks)
    Return,
}

/// A wrapper type for intersection IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntersectionId(pub SimId);

/// A wrapper type for road IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoadId(pub SimId);

/// A wrapper type for car IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CarId(pub SimId);

/// A wrapper type for apartment IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ApartmentId(pub SimId);

/// A wrapper type for factory IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FactoryId(pub SimId);

/// A wrapper type for shop IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShopId(pub SimId);

/// A 3D position in the simulation
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Position {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn distance(&self, other: &Position) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    pub fn lerp(&self, other: &Position, t: f32) -> Position {
        Position {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
        }
    }

    /// Calculate the angle from this position to another (Y-axis rotation)
    pub fn angle_to(&self, other: &Position) -> f32 {
        let dx = other.x - self.x;
        let dz = other.z - self.z;
        let direction_len = (dx * dx + dz * dz).sqrt();
        if direction_len > 0.0 {
            (dx / direction_len).atan2(dz / direction_len)
        } else {
            0.0
        }
    }

    /// Calculate perpendicular offset (right side of direction)
    pub fn perpendicular_offset(&self, other: &Position, offset: f32) -> Position {
        let dx = other.x - self.x;
        let dz = other.z - self.z;
        let len = (dx * dx + dz * dz).sqrt();
        if len > 0.0 {
            // Perpendicular: rotate 90 degrees
            Position {
                x: -dz / len * offset,
                y: 0.0,
                z: dx / len * offset,
            }
        } else {
            Position::new(0.0, 0.0, 0.0)
        }
    }
}

impl Default for Position {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

/// A road segment connecting two intersections
#[derive(Debug, Clone)]
pub struct SimRoad {
    pub id: RoadId,
    pub start_intersection: IntersectionId,
    pub end_intersection: IntersectionId,
    pub length: f32,
    pub angle: f32,
    pub is_two_way: bool,
}

impl SimRoad {
    pub fn new(
        id: RoadId,
        start_intersection: IntersectionId,
        end_intersection: IntersectionId,
        start_pos: &Position,
        end_pos: &Position,
        is_two_way: bool,
    ) -> Self {
        let length = start_pos.distance(end_pos);
        let angle = start_pos.angle_to(end_pos);

        Self {
            id,
            start_intersection,
            end_intersection,
            length,
            angle,
            is_two_way,
        }
    }
}

/// Length of a car in world units
pub const CAR_LENGTH: f32 = 0.5;

/// Distance from intersection to start checking for lock
pub const INTERSECTION_APPROACH_DISTANCE: f32 = 1.0;

/// Safe following distance multiplier for CAR_LENGTH
pub const SAFE_FOLLOWING_MULTIPLIER: f32 = 1.5;
