mod car;
mod house;
mod interface;
mod intersection;
mod road;
mod road_network;
mod stats;
mod world;

use bevy::log::LogPlugin;
use bevy::prelude::*;
use car::CarPlugin;
use house::HousePlugin;
use interface::InterfacePlugin;
use road::RoadPlugin;
use stats::StatsPlugin;
use std::env;
use world::WorldPlugin;

fn main() {
    let headless = env::args().any(|arg| arg == "--headless");

    let mut app = App::new();

    if headless {
        // Headless mode - use headless render plugin
        app.add_plugins(
            MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
                std::time::Duration::from_secs_f64(1.0 / 60.0),
            )),
        )
        .add_plugins((
            bevy::asset::AssetPlugin::default(),
            bevy::scene::ScenePlugin,
        ))
        .add_plugins(LogPlugin {
            filter: "warn,traffic_sim=info".to_string(),
            level: bevy::log::Level::INFO,
            ..default()
        })
        // Register mesh and material asset types for headless mode
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>();
    } else {
        // Normal mode with rendering
        app.add_plugins(
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
        );
    }

    // Add our custom plugins for each game concept
    app.add_plugins((WorldPlugin, RoadPlugin, CarPlugin, HousePlugin, StatsPlugin));

    // Only add interface plugin in non-headless mode
    if !headless {
        app.add_plugins(InterfacePlugin);
    }

    app.add_systems(Update, handle_input).run();
}

/// Handle basic keyboard input
fn handle_input(keyboard: Option<Res<ButtonInput<KeyCode>>>, mut exit: MessageWriter<AppExit>) {
    if let Some(keyboard) = keyboard {
        if keyboard.just_pressed(KeyCode::Escape) {
            exit.write(AppExit::Success);
        }
    }
}
