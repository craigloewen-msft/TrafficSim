mod car;
mod house;
mod interface;
mod intersection;
mod road;
mod road_network;
mod two_way_road;
mod world;

use bevy::prelude::*;
use bevy::log::LogPlugin;
use car::CarPlugin;
use house::HousePlugin;
use interface::InterfacePlugin;
use road::RoadPlugin;
use world::WorldPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(LogPlugin {
                    filter: "warn,traffic_sim=debug".to_string(),
                    level: bevy::log::Level::DEBUG,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Traffic Sim - Bevy Game".into(),
                        resolution: (1280, 720).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        // Add our custom plugins for each game concept
        .add_plugins((WorldPlugin, RoadPlugin, CarPlugin, HousePlugin, InterfacePlugin))
        .add_systems(Update, handle_input)
        .run();
}

/// Handle basic keyboard input
fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}
