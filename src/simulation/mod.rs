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

// Re-export public types for external use
#[allow(unused_imports)]
pub use types::*;
#[allow(unused_imports)]
pub use road_network::SimRoadNetwork;
#[allow(unused_imports)]
pub use intersection::SimIntersection;
#[allow(unused_imports)]
pub use car::{SimCar, CarUpdateResult};
#[allow(unused_imports)]
pub use building::{SimHouse, SimFactory, SimShop};
pub use world::SimWorld;
