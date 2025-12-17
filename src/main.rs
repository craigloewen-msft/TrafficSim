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
#[command(about = "Traffic management game - Build roads and manage deliveries!")]
struct Cli {
    /// Run with the Bevy game engine UI to play the game
    #[arg(long)]
    ui: bool,

    /// Number of simulation ticks to run in test/headless mode
    #[arg(long, default_value = "1000")]
    ticks: u32,

    /// Time delta per tick in seconds (test mode)
    #[arg(long, default_value = "0.1")]
    delta: f32,

    /// Random seed for reproducible simulations (test mode only)
    #[arg(long, default_value = "42")]
    seed: u64,

    /// Display the simulation visually in the CLI with periodic updates
    #[arg(long)]
    cli_display: bool,
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
        println!("===========================================");
        println!("  Traffic Sim - Test Mode");
        println!("===========================================");
        println!("To play the game, run with: cargo run --ui");
        println!("(Note: Requires UI feature enabled)");
        println!("===========================================");
        println!();
        
        if cli.cli_display {
            run_headless_with_display(cli.ticks, cli.delta);
        } else {
            run_headless(cli.ticks, cli.delta, cli.seed);
        }
    }
}

/// Helper function to run simulation with validation
///
/// Runs a simulation for the specified number of ticks and validates
/// that the simulation state remains consistent. Returns validation
/// results and statistics.
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
fn run_simulation_validation(
    ticks: u32,
    delta: f32,
    seed: u64,
) -> (bool, usize, usize, Vec<String>) {
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
    println!("=== SIMULATION RESULTS ===");
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
    }

    // Check: Road network should be intact
    if world.road_network.intersection_count() != initial_intersections
        || world.road_network.road_count() != initial_roads
    {
        errors.push("FAIL: Road network was unexpectedly modified".to_string());
        validation_passed = false;
    }

    // Check: Buildings should be intact
    if world.houses.len() != initial_houses
        || world.factories.len() != initial_factories
        || world.shops.len() != initial_shops
    {
        errors.push("FAIL: Buildings were unexpectedly modified".to_string());
        validation_passed = false;
    }

    (validation_passed, total_deliveries, max_cars_observed, errors)
}

/// Print validation results in a formatted manner
///
/// # Arguments
/// * `validation_passed` - Whether all validation checks passed
/// * `max_cars_observed` - Maximum number of concurrent cars
/// * `errors` - List of error messages
/// * `mode` - Mode prefix for messages ("SIMULATION" or "TEST")
fn print_validation_results_with_mode(
    validation_passed: bool,
    max_cars_observed: usize,
    errors: &[String],
    mode: &str,
) {
    // Print success/failure for each check
    if max_cars_observed == 0 {
        println!("FAIL: No cars were ever spawned during simulation");
    } else {
        println!(
            "PASS: Cars spawned successfully (max: {})",
            max_cars_observed
        );
    }

    if errors.iter().any(|e| e.contains("Road network")) {
        println!("FAIL: Road network was unexpectedly modified");
    } else {
        println!("PASS: Road network integrity maintained");
    }

    if errors.iter().any(|e| e.contains("Building") || e.contains("House") || e.contains("Factory") || e.contains("Shop")) {
        println!("FAIL: Buildings were unexpectedly modified");
    } else {
        println!("PASS: Building integrity maintained");
    }

    // Print any errors
    if !errors.is_empty() {
        println!();
        println!("=== ERRORS ===");
        for error in errors {
            println!("  {}", error);
        }
    }

    println!();
    if validation_passed && errors.is_empty() {
        println!("{} PASSED: All validations succeeded", mode);
    } else {
        println!("{} FAILED: {} error(s) detected", mode, errors.len());
    }
}

/// Print validation results for normal simulation mode
///
/// # Arguments
/// * `validation_passed` - Whether all validation checks passed
/// * `total_deliveries` - Total number of deliveries completed (unused but kept for compatibility)
/// * `max_cars_observed` - Maximum number of concurrent cars
/// * `errors` - List of error messages
fn print_validation_results(
    validation_passed: bool,
    _total_deliveries: usize,
    max_cars_observed: usize,
    errors: &[String],
) {
    print_validation_results_with_mode(validation_passed, max_cars_observed, errors, "SIMULATION");
}

/// Run the simulation in headless mode (no graphics)
///
/// This mode runs the simulation for a fixed number of ticks and validates
/// that the simulation is working correctly. It outputs final statistics
/// and validation results, similar to test mode.
///
/// # Arguments
/// * `ticks` - Number of simulation ticks to run
/// * `delta` - Time delta per tick in seconds
/// * `seed` - Random seed for deterministic simulation
fn run_headless(ticks: u32, delta: f32, seed: u64) {
    println!("Running traffic simulation in headless mode...");
    println!("Ticks: {}, Delta: {}s, Seed: {}", ticks, delta, seed);
    println!();

    let (validation_passed, total_deliveries, max_cars_observed, errors) =
        run_simulation_validation(ticks, delta, seed);

    // Print validation results
    print_validation_results(
        validation_passed,
        total_deliveries,
        max_cars_observed,
        &errors,
    );

    if !validation_passed || !errors.is_empty() {
        std::process::exit(1);
    }
}

/// Run the simulation in headless mode with CLI display
///
/// This mode runs the simulation for a fixed number of ticks and prints
/// periodic summaries to the console with animated map display. It's useful 
/// for visually observing the simulation logic without the overhead of the UI.
///
/// # Arguments
/// * `ticks` - Number of simulation ticks to run
/// * `delta` - Time delta per tick in seconds
fn run_headless_with_display(ticks: u32, delta: f32) {
    println!("Running traffic simulation in headless mode with CLI display...");
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
/// This mode provides a visual interface for the traffic management game:
/// - Build roads and buildings to create delivery networks
/// - Earn money from successful deliveries
/// - Reach the goal to win the game!
fn run_with_ui() {
    use bevy::log::LogPlugin;
    use bevy::prelude::*;

    println!("===========================================");
    println!("  Traffic Management Game");
    println!("===========================================");
    println!();
    println!("🎮 OBJECTIVE:");
    println!("  Complete 50 shop deliveries OR earn $5000");
    println!();
    println!("💰 ECONOMICS:");
    println!("  Starting Budget: $2000");
    println!("  Road: $50 | House: $200 | Factory: $500 | Shop: $300");
    println!("  Earn $10 per worker trip, $50 per shop delivery");
    println!();
    println!("🕹️ CONTROLS:");
    println!("  Camera:");
    println!("    W/A/S/D     - Move camera");
    println!("    Q/E         - Rotate camera around center");
    println!("    Z/X         - Zoom in/out");
    println!("    Click+Drag  - Orbital rotation");
    println!("    ESC         - Exit");
    println!();
    println!("  Building:");
    println!("    1 or Button - Road mode (click two points)");
    println!("    2 or Button - House mode (click to place)");
    println!("    3 or Button - Factory mode (click to place)");
    println!("    4 or Button - Shop mode (click to place)");
    println!();
    println!("💡 TIPS:");
    println!("  • Houses send workers to factories");
    println!("  • Factories produce goods and send trucks to shops");
    println!("  • Shorter routes = faster deliveries = more money!");
    println!("  • Watch your budget - you can't build if bankrupt");
    println!("===========================================");
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
                        title: "Traffic Management Game".into(),
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

    let (validation_passed, total_deliveries, max_cars_observed, errors) =
        run_simulation_validation(ticks, delta, seed);

    // Print validation results (same as headless mode but with "TEST" prefix)
    print_test_validation_results(
        validation_passed,
        total_deliveries,
        max_cars_observed,
        &errors,
    );

    (validation_passed, total_deliveries, max_cars_observed, errors)
}

/// Print test validation results (similar to print_validation_results but for tests)
#[cfg(test)]
fn print_test_validation_results(
    validation_passed: bool,
    _total_deliveries: usize,
    max_cars_observed: usize,
    errors: &[String],
) {
    print_validation_results_with_mode(validation_passed, max_cars_observed, errors, "TEST");
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

    /// Tests that factories correctly implement the simplified delivery logic:
    /// 1. Factories accept workers only when truck is home (not out making deliveries)
    /// 2. Deliveries count no longer affects worker acceptance
    #[test]
    fn test_factory_delivery_logic() {
        use simulation::SimWorld;
        
        println!("Testing factory delivery logic...");
        
        // Create a simple test world
        let mut world = SimWorld::create_test_world();
        
        // Get a factory reference
        let factory_id = *world.factories.keys().next().expect("No factories in test world");
        
        // Initial state: truck should be home, deliveries should be 0
        {
            let factory = world.factories.get(&factory_id).unwrap();
            assert!(factory.truck.is_none(), "Factory should start with truck at home");
            assert_eq!(factory.deliveries_ready, 0, "Factory should start with 0 deliveries");
            assert_eq!(factory.max_deliveries, 2, "Factory max deliveries should be 2");
        }
        
        // Simulate workers completing work to build up deliveries
        // We'll directly manipulate the factory state to test the logic
        {
            let factory = world.factories.get_mut(&factory_id).unwrap();
            // Simulate 2 workers completing work (max deliveries)
            factory.deliveries_ready = 2;
        }
        
        // Verify factory still accepts workers when truck is home (deliveries count doesn't matter)
        {
            let factory = world.factories.get(&factory_id).unwrap();
            assert_eq!(factory.deliveries_ready, 2, "Factory should have 2 deliveries ready");
            
            // With simplified logic, factory SHOULD accept workers even when full, as long as truck is home
            assert!(factory.can_accept_workers(), "Factory should accept workers when truck is home (even with 2/2 deliveries)");
        }
        
        // Take a delivery (simulate truck dispatch)
        {
            let factory = world.factories.get_mut(&factory_id).unwrap();
            let taken = factory.take_delivery();
            assert!(taken, "Should be able to take a delivery");
            assert_eq!(factory.deliveries_ready, 1, "Should have 1 delivery remaining");
        }
        
        // Factory should still be able to accept workers (truck still home)
        {
            let factory = world.factories.get(&factory_id).unwrap();
            assert!(factory.can_accept_workers(), "Factory should accept workers when truck is home");
        }
        
        // Simulate truck being out
        {
            let factory = world.factories.get_mut(&factory_id).unwrap();
            factory.truck = Some(simulation::CarId(simulation::SimId(999)));
        }
        
        // Verify factory won't accept workers when truck is out
        {
            let factory = world.factories.get(&factory_id).unwrap();
            assert!(factory.truck.is_some(), "Factory truck should be out");
            assert!(!factory.can_accept_workers(), "Factory should not accept workers when truck is out");
        }
        
        println!("FACTORY DELIVERY LOGIC TEST PASSED");
    }
}
