//! UI module that visualizes the simulation state using Bevy
//! 
//! This module is purely for visualization - all simulation logic is in the `simulation` module.
//! The UI reads state from `SimWorld` and renders it using Bevy's 3D graphics.

mod components;
mod input;
mod spawner;
mod sync;
mod world;

use bevy::prelude::*;

pub use components::{EntityMappings, SimWorldResource};

use components::*;
use input::{handle_camera_mouse, handle_camera_movement, handle_input};
use spawner::spawn_initial_visuals;
use sync::{sync_cars, tick_simulation, update_factory_indicators, update_shop_indicators};
use world::setup_world;

/// Plugin to register all UI systems
pub struct TrafficSimUIPlugin;

impl Plugin for TrafficSimUIPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimWorldResource>()
            .init_resource::<EntityMappings>()
            .init_resource::<CameraSettings>()
            .add_systems(Startup, (setup_world, spawn_initial_visuals.after(setup_world)))
            .add_systems(FixedUpdate, tick_simulation)
            .add_systems(
                Update,
                (
                    sync_cars,
                    update_factory_indicators,
                    update_shop_indicators,
                    handle_input,
                    handle_camera_movement,
                    handle_camera_mouse,
                ),
            );
    }
}
