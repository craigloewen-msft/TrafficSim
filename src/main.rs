//! Traffic Simulation
//!
//! A traffic simulation that can run in both headless and UI modes.
//! The simulation models cars traveling between houses, factories, and shops.

use traffic_sim::simulation;

#[cfg(feature = "ui")]
use traffic_sim::ui;

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
            run_headless_with_display(cli.ticks, cli.delta, cli.seed);
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
    let initial_apartments = world.apartments.len();
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
                    tick,
                    initial_intersections,
                    world.road_network.intersection_count()
                ));
            }
            if world.road_network.road_count() != initial_roads {
                errors.push(format!(
                    "Tick {}: Road count changed from {} to {}",
                    tick,
                    initial_roads,
                    world.road_network.road_count()
                ));
            }
            if world.apartments.len() != initial_apartments {
                errors.push(format!(
                    "Tick {}: Apartment count changed from {} to {}",
                    tick,
                    initial_apartments,
                    world.apartments.len()
                ));
            }
            if world.factories.len() != initial_factories {
                errors.push(format!(
                    "Tick {}: Factory count changed from {} to {}",
                    tick,
                    initial_factories,
                    world.factories.len()
                ));
            }
            if world.shops.len() != initial_shops {
                errors.push(format!(
                    "Tick {}: Shop count changed from {} to {}",
                    tick,
                    initial_shops,
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
    if world.apartments.len() != initial_apartments
        || world.factories.len() != initial_factories
        || world.shops.len() != initial_shops
    {
        errors.push("FAIL: Buildings were unexpectedly modified".to_string());
        validation_passed = false;
    }

    (
        validation_passed,
        total_deliveries,
        max_cars_observed,
        errors,
    )
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

    if errors.iter().any(|e| {
        e.contains("Building") || e.contains("House") || e.contains("Factory") || e.contains("Shop")
    }) {
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
/// * `seed` - Random seed for deterministic simulation
fn run_headless_with_display(ticks: u32, delta: f32, seed: u64) {
    println!("Running traffic simulation in headless mode with CLI display...");
    println!("Ticks: {}, Delta: {}s, Seed: {}", ticks, delta, seed);

    // Calculate how many ticks equal 1 second of simulation time
    let ticks_per_second = (1.0 / delta).ceil() as u32;
    println!(
        "Running {} ticks per second (simulated time)",
        ticks_per_second
    );
    println!();

    let mut world = simulation::SimWorld::create_test_world_with_seed(seed);

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
    use ui::UI_STARTING_BUDGET;
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
    println!("  Starting Budget: ${} (UI sandbox)", UI_STARTING_BUDGET);
    println!("  Road: $50 | House: $200 | Factory: $500 | Shop: $300");
    println!("  Earn $10 per worker trip, $50 per shop delivery");
    println!("  Start with a blank map so you can design your own layout");
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
fn run_simulation_test(ticks: u32, delta: f32, seed: u64) -> (bool, usize, usize, Vec<String>) {
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

    (
        validation_passed,
        total_deliveries,
        max_cars_observed,
        errors,
    )
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
        let factory_id = *world
            .factories
            .keys()
            .next()
            .expect("No factories in test world");

        // Initial state: truck should be home, deliveries should be 0
        {
            let factory = world.factories.get(&factory_id).unwrap();
            assert!(
                factory.truck.is_none(),
                "Factory should start with truck at home"
            );
            assert_eq!(
                factory.deliveries_ready, 0,
                "Factory should start with 0 deliveries"
            );
            assert_eq!(
                factory.max_deliveries, 2,
                "Factory max deliveries should be 2"
            );
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
            assert_eq!(
                factory.deliveries_ready, 2,
                "Factory should have 2 deliveries ready"
            );

            // With simplified logic, factory SHOULD accept workers even when full, as long as truck is home
            assert!(
                factory.can_accept_workers(),
                "Factory should accept workers when truck is home (even with 2/2 deliveries)"
            );
        }

        // Take a delivery (simulate truck dispatch)
        {
            let factory = world.factories.get_mut(&factory_id).unwrap();
            let taken = factory.take_delivery();
            assert!(taken, "Should be able to take a delivery");
            assert_eq!(
                factory.deliveries_ready, 1,
                "Should have 1 delivery remaining"
            );
        }

        // Factory should still be able to accept workers (truck still home)
        {
            let factory = world.factories.get(&factory_id).unwrap();
            assert!(
                factory.can_accept_workers(),
                "Factory should accept workers when truck is home"
            );
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
            assert!(
                !factory.can_accept_workers(),
                "Factory should not accept workers when truck is out"
            );
        }

        println!("FACTORY DELIVERY LOGIC TEST PASSED");
    }

    /// Tests that traffic-aware pathfinding correctly factors in congestion
    ///
    /// This test creates a simple road network with two parallel routes and
    /// verifies that pathfinding prefers the less congested route when traffic
    /// is present.
    #[test]
    fn test_traffic_aware_pathfinding() {
        use simulation::{Position, SimWorld};
        use ordered_float::OrderedFloat;

        println!("Testing traffic-aware pathfinding...");

        // Create a simple world
        let mut world = SimWorld::new_with_seed(42);

        // Create a diamond-shaped road network:
        //
        //        B
        //       / \
        //      A   D
        //       \ /
        //        C
        //
        // Two routes from A to D: A -> B -> D (top) and A -> C -> D (bottom)
        // Both routes have the same length

        let a = world.add_intersection(Position::new(0.0, 0.0, 0.0));
        let b = world.add_intersection(Position::new(5.0, 0.0, -5.0));
        let c = world.add_intersection(Position::new(5.0, 0.0, 5.0));
        let d = world.add_intersection(Position::new(10.0, 0.0, 0.0));

        // Add roads (two-way)
        let (road_ab, _) = world.add_two_way_road(a, b).unwrap();
        let (road_bd, _) = world.add_two_way_road(b, d).unwrap();
        let (road_ac, _) = world.add_two_way_road(a, c).unwrap();
        let (road_cd, _) = world.add_two_way_road(c, d).unwrap();

        // Initially, with no traffic, both routes should be equivalent
        // The pathfinding will pick one (it may prefer one based on graph order)
        let initial_path = world.road_network.find_path(a, d);
        assert!(initial_path.is_some(), "Should find a path from A to D");
        let initial_path = initial_path.unwrap();
        println!(
            "Initial path (no traffic): {:?}",
            initial_path
        );

        // Now add traffic to the top route (A -> B -> D)
        // Simulate 5 cars on road A -> B
        let car_id = simulation::CarId(simulation::SimId(100));
        world
            .road_network
            .update_car_road_position(
                car_id,
                road_ab,
                OrderedFloat(1.0),
                false,
                None,
                OrderedFloat(0.0),
            )
            .unwrap();

        let car_id2 = simulation::CarId(simulation::SimId(101));
        world
            .road_network
            .update_car_road_position(
                car_id2,
                road_ab,
                OrderedFloat(2.0),
                false,
                None,
                OrderedFloat(0.0),
            )
            .unwrap();

        let car_id3 = simulation::CarId(simulation::SimId(102));
        world
            .road_network
            .update_car_road_position(
                car_id3,
                road_ab,
                OrderedFloat(3.0),
                false,
                None,
                OrderedFloat(0.0),
            )
            .unwrap();

        let car_id4 = simulation::CarId(simulation::SimId(103));
        world
            .road_network
            .update_car_road_position(
                car_id4,
                road_bd,
                OrderedFloat(1.0),
                false,
                None,
                OrderedFloat(0.0),
            )
            .unwrap();

        let car_id5 = simulation::CarId(simulation::SimId(104));
        world
            .road_network
            .update_car_road_position(
                car_id5,
                road_bd,
                OrderedFloat(2.0),
                false,
                None,
                OrderedFloat(0.0),
            )
            .unwrap();

        // Verify traffic is counted correctly
        let count_ab = world.road_network.get_car_count_on_road(road_ab);
        let count_bd = world.road_network.get_car_count_on_road(road_bd);
        let count_ac = world.road_network.get_car_count_on_road(road_ac);
        let count_cd = world.road_network.get_car_count_on_road(road_cd);

        println!("Traffic counts:");
        println!("  Road A -> B: {} cars", count_ab);
        println!("  Road B -> D: {} cars", count_bd);
        println!("  Road A -> C: {} cars", count_ac);
        println!("  Road C -> D: {} cars", count_cd);

        assert_eq!(count_ab, 3, "Road A->B should have 3 cars");
        assert_eq!(count_bd, 2, "Road B->D should have 2 cars");
        assert_eq!(count_ac, 0, "Road A->C should have 0 cars");
        assert_eq!(count_cd, 0, "Road C->D should have 0 cars");

        // Find path again - should prefer the bottom route (A -> C -> D) due to traffic
        let traffic_path = world.road_network.find_path(a, d);
        assert!(traffic_path.is_some(), "Should still find a path from A to D");
        let traffic_path = traffic_path.unwrap();
        println!("Path with traffic on top route: {:?}", traffic_path);

        // The path should now prefer the bottom route (going through C)
        assert!(
            traffic_path.contains(&c),
            "Path should prefer the less congested route through C, got: {:?}",
            traffic_path
        );

        println!("TRAFFIC-AWARE PATHFINDING TEST PASSED");
    }

    /// Tests that traffic weight calculation works correctly
    #[test]
    fn test_traffic_weight_calculation() {
        use simulation::{Position, SimWorld};
        use ordered_float::OrderedFloat;

        println!("Testing traffic weight calculation...");

        let mut world = SimWorld::new_with_seed(42);

        // Create a simple road
        let a = world.add_intersection(Position::new(0.0, 0.0, 0.0));
        let b = world.add_intersection(Position::new(10.0, 0.0, 0.0));
        let (road_id, _) = world.add_two_way_road(a, b).unwrap();

        // Get base weight (road length * 100 = 10.0 * 100 = 1000)
        let base_weight = 1000u32; // 10.0 road length * 100

        // No traffic: weight should equal base weight
        let weight_no_traffic = world.road_network.calculate_traffic_weight(road_id, base_weight);
        assert_eq!(
            weight_no_traffic, base_weight,
            "With no traffic, weight should equal base weight"
        );

        // Add 1 car: weight should be base * (1 + 0.2) = base * 1.2
        let car_id = simulation::CarId(simulation::SimId(100));
        world
            .road_network
            .update_car_road_position(
                car_id,
                road_id,
                OrderedFloat(1.0),
                false,
                None,
                OrderedFloat(0.0),
            )
            .unwrap();

        let weight_1_car = world.road_network.calculate_traffic_weight(road_id, base_weight);
        let expected_weight_1 = (base_weight as f32 * 1.2) as u32;
        assert_eq!(
            weight_1_car, expected_weight_1,
            "With 1 car, weight should be {} (got {})",
            expected_weight_1, weight_1_car
        );

        // Add 4 more cars (total 5): weight should be base * (1 + 5*0.2) = base * 2.0
        for i in 1..5 {
            let car = simulation::CarId(simulation::SimId(100 + i));
            world
                .road_network
                .update_car_road_position(
                    car,
                    road_id,
                    OrderedFloat(i as f32 + 1.0),
                    false,
                    None,
                    OrderedFloat(0.0),
                )
                .unwrap();
        }

        let weight_5_cars = world.road_network.calculate_traffic_weight(road_id, base_weight);
        let expected_weight_5 = (base_weight as f32 * 2.0) as u32;
        assert_eq!(
            weight_5_cars, expected_weight_5,
            "With 5 cars, weight should be {} (got {})",
            expected_weight_5, weight_5_cars
        );

        // Add many more cars to test the max multiplier cap (should cap at 3.0)
        for i in 5..20 {
            let car = simulation::CarId(simulation::SimId(100 + i));
            world
                .road_network
                .update_car_road_position(
                    car,
                    road_id,
                    OrderedFloat(i as f32 + 1.0),
                    false,
                    None,
                    OrderedFloat(0.0),
                )
                .unwrap();
        }

        let weight_many_cars = world.road_network.calculate_traffic_weight(road_id, base_weight);
        let expected_max_weight = (base_weight as f32 * 3.0) as u32;
        assert_eq!(
            weight_many_cars, expected_max_weight,
            "With many cars, weight should cap at {} (got {})",
            expected_max_weight, weight_many_cars
        );

        println!("Car count on road: {}", world.road_network.get_car_count_on_road(road_id));
        println!("Weight with 0 cars: {}", base_weight);
        println!("Weight with 1 car: {}", expected_weight_1);
        println!("Weight with 5 cars: {}", expected_weight_5);
        println!("Weight with many cars (capped): {}", expected_max_weight);

        println!("TRAFFIC WEIGHT CALCULATION TEST PASSED");
    }

    /// Tests traffic density calculation
    #[test]
    fn test_traffic_density() {
        use simulation::{Position, SimWorld};
        use ordered_float::OrderedFloat;

        println!("Testing traffic density calculation...");

        let mut world = SimWorld::new_with_seed(42);

        // Create a 10-unit long road
        let a = world.add_intersection(Position::new(0.0, 0.0, 0.0));
        let b = world.add_intersection(Position::new(10.0, 0.0, 0.0));
        let (road_id, _) = world.add_two_way_road(a, b).unwrap();

        // No traffic: density should be 0
        let density_no_traffic = world.road_network.calculate_traffic_density(road_id);
        assert!(
            density_no_traffic.abs() < f32::EPSILON,
            "Density with no traffic should be 0"
        );

        // Add 5 cars on a 10-unit road: density = 5/10 = 0.5
        for i in 0..5 {
            let car = simulation::CarId(simulation::SimId(100 + i));
            world
                .road_network
                .update_car_road_position(
                    car,
                    road_id,
                    OrderedFloat(i as f32 * 2.0),
                    false,
                    None,
                    OrderedFloat(0.0),
                )
                .unwrap();
        }

        let density_5_cars = world.road_network.calculate_traffic_density(road_id);
        assert!(
            (density_5_cars - 0.5).abs() < 0.01,
            "Density with 5 cars on 10-unit road should be ~0.5, got {}",
            density_5_cars
        );

        println!("Density with 0 cars: {}", density_no_traffic);
        println!("Density with 5 cars on 10-unit road: {}", density_5_cars);

        println!("TRAFFIC DENSITY TEST PASSED");
    }
}
