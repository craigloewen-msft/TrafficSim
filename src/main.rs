//! Traffic Simulation
//!
//! A traffic simulation that can run in both headless and UI modes.
//! The simulation models cars traveling between houses, factories, and shops.

mod simulation;

#[cfg(feature = "ui")]
mod ui;

use clap::Parser;

#[derive(Parser)]
#[command(name = "traffic_sim")]
#[command(about = "Traffic simulation with optional UI")]
struct Cli {
    /// Run with the Bevy game engine UI
    #[arg(long)]
    ui: bool,

    /// Number of simulation ticks to run in headless mode
    #[arg(long, default_value = "9000")]
    ticks: u32,

    /// Time delta per tick in seconds
    #[arg(long, default_value = "0.1")]
    delta: f32,

    /// Random seed for reproducible simulations (test mode only)
    #[arg(long, default_value = "42")]
    seed: u64,
}

fn main() {
    let cli = Cli::parse();

    if cli.ui {
        #[cfg(feature = "ui")]
        {
            run_with_ui();
        }
        #[cfg(not(feature = "ui"))]
        {
            eprintln!("Error: UI feature is not enabled. Rebuild with --features ui");
            std::process::exit(1);
        }
    } else {
        run_headless(cli.ticks, cli.delta);
    }
}

/// Run the simulation in headless mode (no graphics)
///
/// This mode runs the simulation for a fixed number of ticks and prints
/// periodic summaries to the console. It's useful for testing and debugging
/// the simulation logic without the overhead of the UI.
///
/// # Arguments
/// * `ticks` - Number of simulation ticks to run
/// * `delta` - Time delta per tick in seconds
fn run_headless(ticks: u32, delta: f32) {
    println!("Running traffic simulation in headless mode...");
    println!("Ticks: {}, Delta: {}s", ticks, delta);

    // Calculate how many ticks equal 1 second of simulation time
    let ticks_per_second = (1.0 / delta).ceil() as u32;
    println!(
        "Running {} ticks per second (simulated time)",
        ticks_per_second
    );
    println!();

    let mut world = simulation::SimWorld::create_test_world();

    println!("Initial state:");
    world.print_summary();
    world.draw_map();
    println!();

    // Run simulation
    let mut tick = 0;
    while tick < ticks {
        // Run ticks_per_second ticks (or remaining ticks if fewer)
        let ticks_to_run = ticks_per_second.min(ticks - tick);

        for _ in 0..ticks_to_run {
            tick += 1;
            world.tick(delta);
        }

        // Print summary after running 1 second worth of ticks
        println!(
            "--- After tick {} ({:.1}s simulated time) ---",
            tick,
            tick as f32 * delta
        );
        world.print_summary();
        world.draw_map();
        println!();

        if tick < ticks {
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }

    println!("=== Final State ===");
    world.print_summary();
    world.draw_map();
}

#[cfg(feature = "ui")]
/// Run the simulation with the Bevy game engine UI
///
/// This mode provides a visual interface for the simulation with:
/// - 3D rendering of roads, buildings, and cars
/// - Camera controls for navigation
/// - Interactive building placement
/// - Real-time visualization of traffic flow
fn run_with_ui() {
    use bevy::log::LogPlugin;
    use bevy::prelude::*;

    println!("Starting Traffic Sim UI...");
    println!();
    println!("Camera Controls:");
    println!("  W/A/S/D     - Move camera");
    println!("  Q/E         - Rotate camera around center");
    println!("  Z/X         - Zoom in/out");
    println!("  Click+Drag  - Orbital rotation");
    println!("  ESC         - Exit");
    println!();
    println!("Building Controls:");
    println!("  1           - Road mode (click two points to create a road)");
    println!("  2           - House mode (click to place)");
    println!("  3           - Factory mode (click to place)");
    println!("  4           - Shop mode (click to place)");
    println!("  Click buttons at bottom of screen to toggle modes");
    println!();
    println!("Building snaps to nearby intersections and roads.");
    println!();

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
        .add_plugins(ui::TrafficSimUIPlugin)
        .run();
}

/// Helper function to run a simulation test with validation
///
/// This function runs a deterministic simulation with the given parameters
/// and validates that the simulation is working correctly. It tracks various
/// metrics and performs integrity checks on the simulation state.
///
/// # Arguments
/// * `ticks` - Number of simulation ticks to run
/// * `delta` - Time delta per tick in seconds
/// * `seed` - Random seed for deterministic simulation
///
/// # Returns
/// A tuple containing:
/// * `validation_passed` - Whether all validation checks passed
/// * `total_deliveries` - Total number of deliveries completed
/// * `max_cars_observed` - Maximum number of concurrent cars
/// * `errors` - List of error messages (if any)
#[cfg(test)]
fn run_simulation_test(
    ticks: u32,
    delta: f32,
    seed: u64,
) -> (bool, usize, usize, Vec<String>) {
    println!("Running traffic simulation in TEST mode...");
    println!("Ticks: {}, Delta: {}s, Seed: {}", ticks, delta, seed);
    println!();

    let mut world = simulation::SimWorld::create_test_world_with_seed(seed);

    // Track initial state for validation
    let initial_houses = world.houses.len();
    let initial_factories = world.factories.len();
    let initial_shops = world.shops.len();
    let initial_intersections = world.road_network.intersection_count();
    let initial_roads = world.road_network.road_count();

    let mut max_cars_observed = 0usize;
    let mut errors: Vec<String> = Vec::new();

    // Run simulation without delays
    for tick in 1..=ticks {
        world.tick(delta);

        // Track maximum concurrent cars
        max_cars_observed = max_cars_observed.max(world.cars.len());

        // Validate simulation state periodically
        if tick % 100 == 0 {
            // Check for structural integrity
            if world.road_network.intersection_count() != initial_intersections {
                errors.push(format!(
                    "Tick {}: Intersection count changed from {} to {}",
                    tick, initial_intersections,
                    world.road_network.intersection_count()
                ));
            }
            if world.road_network.road_count() != initial_roads {
                errors.push(format!(
                    "Tick {}: Road count changed from {} to {}",
                    tick, initial_roads,
                    world.road_network.road_count()
                ));
            }
            if world.houses.len() != initial_houses {
                errors.push(format!(
                    "Tick {}: House count changed from {} to {}",
                    tick, initial_houses,
                    world.houses.len()
                ));
            }
            if world.factories.len() != initial_factories {
                errors.push(format!(
                    "Tick {}: Factory count changed from {} to {}",
                    tick, initial_factories,
                    world.factories.len()
                ));
            }
            if world.shops.len() != initial_shops {
                errors.push(format!(
                    "Tick {}: Shop count changed from {} to {}",
                    tick, initial_shops,
                    world.shops.len()
                ));
            }
        }
    }

    // Calculate total deliveries
    let total_deliveries: usize = world.shops.values().map(|s| s.cars_received).sum();

    // Print test results
    println!("=== TEST RESULTS ===");
    println!("Simulation time: {:.2}s", world.time);
    println!("Max concurrent cars: {}", max_cars_observed);
    println!("Total deliveries to shops: {}", total_deliveries);
    println!("Final car count: {}", world.cars.len());
    println!();

    // Validation checks
    let mut validation_passed = true;

    // Check: Cars should have spawned during simulation
    if max_cars_observed == 0 {
        errors.push("FAIL: No cars were ever spawned during simulation".to_string());
        validation_passed = false;
    } else {
        println!(
            "PASS: Cars spawned successfully (max: {})",
            max_cars_observed
        );
    }

    // Check: Road network should be intact
    if world.road_network.intersection_count() == initial_intersections
        && world.road_network.road_count() == initial_roads
    {
        println!("PASS: Road network integrity maintained");
    } else {
        errors.push("FAIL: Road network was unexpectedly modified".to_string());
        validation_passed = false;
    }

    // Check: Buildings should be intact
    if world.houses.len() == initial_houses
        && world.factories.len() == initial_factories
        && world.shops.len() == initial_shops
    {
        println!("PASS: Building integrity maintained");
    } else {
        errors.push("FAIL: Buildings were unexpectedly modified".to_string());
        validation_passed = false;
    }

    // Print any errors
    if !errors.is_empty() {
        println!();
        println!("=== ERRORS ===");
        for error in &errors {
            println!("  {}", error);
        }
    }

    println!();
    if validation_passed && errors.is_empty() {
        println!("TEST PASSED: All validations succeeded");
    } else {
        println!("TEST FAILED: {} error(s) detected", errors.len());
    }

    (validation_passed, total_deliveries, max_cars_observed, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimum number of deliveries expected in a 1000-tick simulation
    const MIN_EXPECTED_DELIVERIES: usize = 3;

    /// Tests that the simulation produces deliveries within expected thresholds.
    ///
    /// This test validates basic simulation functionality by running 1000 ticks
    /// and ensuring that cars spawn, the road network remains intact, and
    /// a reasonable number of deliveries are completed.
    #[test]
    fn test_simulation_basic() {
        let ticks = 1000;
        let delta = 0.1;
        let seed = 42;

        let (validation_passed, total_deliveries, _max_cars, errors) =
            run_simulation_test(ticks, delta, seed);

        // Assert basic validation passed
        assert!(
            validation_passed && errors.is_empty(),
            "Simulation validation failed"
        );

        // Assert reasonable number of deliveries for 1000 ticks
        // We expect at least MIN_EXPECTED_DELIVERIES to ensure the simulation is functioning
        // Note: Some non-determinism exists even with seeding due to HashMap iteration order
        assert!(
            total_deliveries >= MIN_EXPECTED_DELIVERIES,
            "Expected at least {} deliveries in 1000 ticks, got {}. The simulation may not be functioning properly.",
            MIN_EXPECTED_DELIVERIES,
            total_deliveries
        );

        println!(
            "\nDELIVERY TEST PASSED: {} deliveries completed (>= {} expected)",
            total_deliveries, MIN_EXPECTED_DELIVERIES
        );
    }
}
