//! Standalone traffic simulation module
//! 
//! This module contains all the core traffic simulation logic that can run
//! independently of the Bevy game engine. It can be tested via console
//! without needing to boot up the full game.

mod types;
mod road_network;
mod intersection;
mod car;
mod building;
mod world;

pub use types::*;
pub use road_network::SimRoadNetwork;
pub use intersection::SimIntersection;
pub use car::{SimCar, CarUpdateResult};
pub use building::{SimHouse, SimFactory, SimShop};
pub use world::SimWorld;
