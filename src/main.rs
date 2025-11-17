mod car;
mod house;
mod interface;
mod intersection;
mod road;
mod road_network;
mod world;

use bevy::prelude::*;
use bevy::log::LogPlugin;
use car::CarPlugin;
use house::HousePlugin;
use interface::InterfacePlugin;
use road::RoadPlugin;
use world::WorldPlugin;
use std::env;

fn main() {
    let headless = env::args().any(|arg| arg == "--headless");
    
    let mut app = App::new();
    
    if headless {
        // Headless mode - use headless render plugin
        app.add_plugins(
            MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
                std::time::Duration::from_secs_f64(1.0 / 60.0),
            ))
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
    app.add_plugins((WorldPlugin, RoadPlugin, CarPlugin, HousePlugin));
    
    // Only add interface plugin in non-headless mode
    if !headless {
        app.add_plugins(InterfacePlugin);
    }
    
    app.add_systems(Update, handle_input)
        .insert_resource(SimulationStats::default())
        .add_systems(Update, update_simulation_stats)
        .add_systems(Update, check_simulation_exit)
        .run();
}

/// Resource to track simulation statistics
#[derive(Resource, Default)]
pub struct SimulationStats {
    pub total_cars_spawned: u32,
    pub total_cars_completed: u32,
    pub active_cars: u32,
    pub total_intersections: u32,
    pub total_roads: u32,
    pub elapsed_time: f32,
}

/// Update simulation statistics
fn update_simulation_stats(
    time: Res<Time>,
    mut stats: ResMut<SimulationStats>,
    car_query: Query<&car::Car>,
    road_query: Query<&road::Road>,
    intersection_query: Query<&intersection::Intersection>,
) {
    stats.elapsed_time += time.delta_secs();
    stats.active_cars = car_query.iter().count() as u32;
    stats.total_roads = road_query.iter().count() as u32;
    stats.total_intersections = intersection_query.iter().count() as u32;
}

/// Check if simulation should exit in headless mode
fn check_simulation_exit(
    stats: Res<SimulationStats>,
    mut exit: MessageWriter<AppExit>,
) {
    // In headless mode, exit after 30 seconds or when all cars have completed
    let headless = env::args().any(|arg| arg == "--headless");
    if headless && (stats.elapsed_time > 30.0 || (stats.total_cars_spawned > 0 && stats.active_cars == 0)) {
        info!("=== SIMULATION COMPLETE ===");
        info!("Elapsed time: {:.2}s", stats.elapsed_time);
        info!("Total cars spawned: {}", stats.total_cars_spawned);
        info!("Total cars completed: {}", stats.total_cars_completed);
        info!("Active cars: {}", stats.active_cars);
        info!("Total intersections: {}", stats.total_intersections);
        info!("Total roads: {}", stats.total_roads);
        info!("Success rate: {:.1}%", if stats.total_cars_spawned > 0 { 
            (stats.total_cars_completed as f32 / stats.total_cars_spawned as f32) * 100.0 
        } else { 
            0.0 
        });
        exit.write(AppExit::Success);
    }
}

/// Handle basic keyboard input
fn handle_input(
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut exit: MessageWriter<AppExit>,
) {
    if let Some(keyboard) = keyboard {
        if keyboard.just_pressed(KeyCode::Escape) {
            exit.write(AppExit::Success);
        }
    }
}
