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

    /// Run in test mode: quick, reproducible simulation with validation
    #[arg(long)]
    test: bool,

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

    if cli.test {
        let success = run_test_simulation(cli.ticks, cli.delta, cli.seed);
        std::process::exit(if success { 0 } else { 1 });
    } else if cli.ui {
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

/// Run the simulation in test mode (deterministic, fast, with validation)
/// Returns true if all validations pass, false otherwise
fn run_test_simulation(ticks: u32, delta: f32, seed: u64) -> bool {
    println!("Running traffic simulation in TEST mode...");
    println!("Ticks: {}, Delta: {}s, Seed: {}", ticks, delta, seed);
    println!();

    let mut world = simulation::SimWorld::create_test_world_with_seed(seed);

    // Track metrics for validation
    let initial_houses = world.houses.len();
    let initial_factories = world.factories.len();
    let initial_shops = world.shops.len();
    let initial_intersections = world.road_network.intersection_count();
    let initial_roads = world.road_network.road_count();

    let mut max_cars_observed = 0usize;
    let mut total_deliveries = 0usize;
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
            if world.houses.len() != initial_houses {
                errors.push(format!(
                    "Tick {}: House count changed from {} to {}",
                    tick,
                    initial_houses,
                    world.houses.len()
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
    for shop in world.shops.values() {
        total_deliveries += shop.cars_received;
    }

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

    // Check: For longer runs, we expect some deliveries
    if ticks >= 100 && total_deliveries == 0 {
        errors.push("FAIL: No deliveries completed (simulation may be stuck)".to_string());
        validation_passed = false;
    } else if ticks >= 100 {
        println!(
            "PASS: Deliveries completed successfully ({})",
            total_deliveries
        );
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
        true
    } else {
        println!("TEST FAILED: {} error(s) detected", errors.len());
        false
    }
}

#[cfg(feature = "ui")]
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
