//! Standalone traffic simulation module
//!
//! This module contains all the core traffic simulation logic that can run
//! independently of the Bevy game engine. It can be tested via console
//! without needing to boot up the full game.

mod building;
mod car;
mod factory;
mod game_state;
mod intersection;
mod road_network;
mod types;
mod world;

// Re-export public types for external use
// These may not be used within this crate but are part of the public API
#[allow(unused_imports)]
pub use building::{SimFactory, SimHouse, SimShop};
#[allow(unused_imports)]
pub use car::{CarUpdateResult, SimCar};
#[allow(unused_imports)]
pub use factory::{FACTORY_WORK_TIME};
#[allow(unused_imports)]
pub use game_state::{
    GameState, COST_FACTORY, COST_HOUSE, COST_ROAD, COST_SHOP, GOAL_DELIVERIES, GOAL_MONEY,
    REVENUE_SHOP_DELIVERY, REVENUE_WORKER_DELIVERY, STARTING_BUDGET,
};
#[allow(unused_imports)]
pub use intersection::SimIntersection;
#[allow(unused_imports)]
pub use road_network::SimRoadNetwork;
#[allow(unused_imports)]
pub use types::{
    CarId, FactoryId, HouseId, IntersectionId, Position, RoadId, ShopId, SimId, SimRoad, TripType,
    VehicleType, CAR_LENGTH, INTERSECTION_APPROACH_DISTANCE, SAFE_FOLLOWING_MULTIPLIER,
};
pub use world::SimWorld;
