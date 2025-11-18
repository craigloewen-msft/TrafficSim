use crate::{car, intersection, road};
use bevy::log::info;
use bevy::prelude::*;
use std::env;

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
fn check_simulation_exit(stats: Res<SimulationStats>, mut exit: MessageWriter<AppExit>) {
    // In headless mode, exit after 30 seconds or when all cars have completed
    let headless = env::args().any(|arg| arg == "--headless");
    if headless
        && (stats.elapsed_time > 30.0 || (stats.total_cars_spawned > 0 && stats.active_cars == 0))
    {
        info!("=== SIMULATION COMPLETE ===");
        info!("Elapsed time: {:.2}s", stats.elapsed_time);
        info!("Total cars spawned: {}", stats.total_cars_spawned);
        info!("Total cars completed: {}", stats.total_cars_completed);
        info!("Active cars: {}", stats.active_cars);
        info!("Total intersections: {}", stats.total_intersections);
        info!("Total roads: {}", stats.total_roads);
        info!(
            "Success rate: {:.1}%",
            if stats.total_cars_spawned > 0 {
                (stats.total_cars_completed as f32 / stats.total_cars_spawned as f32) * 100.0
            } else {
                0.0
            }
        );
        exit.write(AppExit::Success);
    }
}

/// Plugin to register all statistics-related systems
pub struct StatsPlugin;

impl Plugin for StatsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimulationStats::default())
            .add_systems(Update, update_simulation_stats)
            .add_systems(Update, check_simulation_exit);
    }
}
