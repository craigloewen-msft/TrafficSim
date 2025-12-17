//! UI module that visualizes the simulation state using Bevy
//!
//! This module is purely for visualization - all simulation logic is in the `simulation` module.
//! The UI reads state from `SimWorld` and renders it using Bevy's 3D graphics.

mod building;
mod components;
mod input;
pub mod spawner;
mod sync;
mod world;

use bevy::prelude::*;

pub use components::{EntityMappings, SimWorldResource};

use building::{
    handle_build_buttons, handle_build_keyboard, handle_placement_click, setup_building_ui,
    update_button_borders, update_cursor_position, update_ghost_preview,
};
use components::*;
use input::{handle_camera_mouse, handle_camera_movement, handle_input};
use spawner::spawn_initial_visuals;
use sync::{
    sync_cars, tick_simulation, update_factory_delivery_indicators, update_factory_indicators,
    update_global_demand_text, update_house_indicators, update_shop_indicators,
};
use world::setup_world;

/// Plugin to register all UI systems
pub struct TrafficSimUIPlugin;

impl Plugin for TrafficSimUIPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimWorldResource>()
            .init_resource::<EntityMappings>()
            .init_resource::<CameraSettings>()
            .init_resource::<BuildingState>()
            .add_systems(
                Startup,
                (
                    setup_world,
                    spawn_initial_visuals.after(setup_world),
                    setup_building_ui,
                ),
            )
            .add_systems(FixedUpdate, tick_simulation)
            .add_systems(
                Update,
                (
                    sync_cars,
                    update_factory_indicators,
                    update_house_indicators,
                    update_factory_delivery_indicators,
                    update_shop_indicators,
                    update_global_demand_text,
                    handle_input,
                    handle_camera_movement,
                    handle_camera_mouse,
                    handle_build_buttons,
                    handle_build_keyboard,
                    update_cursor_position,
                    update_ghost_preview,
                    handle_placement_click,
                    update_button_borders,
                ),
            );
    }
}
