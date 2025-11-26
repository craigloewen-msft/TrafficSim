//! UI components and resources for linking Bevy entities to simulation state

use bevy::prelude::*;
use std::collections::HashMap;

use crate::simulation::{
    CarId, FactoryId, HouseId, IntersectionId, Position, RoadId, ShopId, SimWorld,
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

/// Marker component for the main camera
#[derive(Component)]
pub struct MainCamera;

/// Resource to control camera movement settings
#[derive(Resource)]
pub struct CameraSettings {
    pub movement_speed: f32,
    pub rotation_speed: f32,
    pub zoom_speed: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            movement_speed: 50.0,
            rotation_speed: 1.0,
            zoom_speed: 30.0,
        }
    }
}

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

/// Building mode types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuildingMode {
    #[default]
    None,
    Road,
    House,
    Factory,
    Shop,
}

/// State for the building system
#[derive(Resource)]
pub struct BuildingState {
    /// Current building mode
    pub mode: BuildingMode,
    /// First point for road placement (when in Road mode)
    pub road_start: Option<Position>,
    /// Current mouse position on ground plane
    pub cursor_position: Option<Position>,
    /// Snapped position (if near an intersection or road)
    pub snapped_position: Option<Position>,
    /// Distance threshold for snapping
    pub snap_distance: f32,
}

impl Default for BuildingState {
    fn default() -> Self {
        Self {
            mode: BuildingMode::None,
            road_start: None,
            cursor_position: None,
            snapped_position: None,
            snap_distance: 2.0,
        }
    }
}

/// Marker for ghost/preview entities
#[derive(Component)]
pub struct GhostPreview;

/// Marker for UI buttons
#[derive(Component)]
pub struct BuildModeButton(pub BuildingMode);

/// Marker for global demand UI text elements
#[derive(Component)]
pub enum GlobalDemandText {
    /// Factories waiting for workers
    FactoriesWaiting,
    /// Shops waiting for products
    ShopsWaiting,
    /// Houses waiting (with available cars but no demand)
    HousesWaiting,
}
