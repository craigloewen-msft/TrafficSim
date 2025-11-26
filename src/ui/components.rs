//! UI components and resources for linking Bevy entities to simulation state

use bevy::prelude::*;
use std::collections::HashMap;

use crate::simulation::{
    CarId, FactoryId, HouseId, IntersectionId, RoadId, ShopId, SimWorld,
};

/// Resource wrapper for the simulation world
#[derive(Resource)]
pub struct SimWorldResource(pub SimWorld);

impl Default for SimWorldResource {
    fn default() -> Self {
        Self(SimWorld::create_test_world())
    }
}

/// Marker component for ground plane
#[derive(Component)]
pub struct Ground;

/// Marker for entities synced from simulation
#[derive(Component)]
pub struct SimSynced;

/// Links a Bevy entity to a simulation intersection
#[derive(Component)]
pub struct IntersectionLink(pub IntersectionId);

/// Links a Bevy entity to a simulation road
#[derive(Component)]
pub struct RoadLink(pub RoadId);

/// Links a Bevy entity to a simulation car
#[derive(Component)]
pub struct CarLink(pub CarId);

/// Links a Bevy entity to a simulation house
#[derive(Component)]
pub struct HouseLink(pub HouseId);

/// Links a Bevy entity to a simulation factory
#[derive(Component)]
pub struct FactoryLink(pub FactoryId);

/// Links a Bevy entity to a simulation shop
#[derive(Component)]
pub struct ShopLink(pub ShopId);

/// Component to mark the visual demand indicator entity
#[derive(Component)]
pub struct DemandIndicator;

/// Resource to track Bevy entities mapped to simulation entities
#[derive(Resource, Default)]
pub struct EntityMappings {
    pub intersections: HashMap<IntersectionId, Entity>,
    pub roads: HashMap<RoadId, Entity>,
    pub cars: HashMap<CarId, Entity>,
    pub houses: HashMap<HouseId, Entity>,
    pub factories: HashMap<FactoryId, Entity>,
    pub shops: HashMap<ShopId, Entity>,
}
