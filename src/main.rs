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
    #[arg(long, default_value = "1000")]
    ticks: u32,

    /// Time delta per tick in seconds
    #[arg(long, default_value = "0.1")]
    delta: f32,
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
fn run_headless(ticks: u32, delta: f32) {
    println!("Running traffic simulation in headless mode...");
    println!("Ticks: {}, Delta: {}s", ticks, delta);
    
    // Calculate how many ticks equal 1 second of simulation time
    let ticks_per_second = (1.0 / delta).ceil() as u32;
    println!("Running {} ticks per second (simulated time)", ticks_per_second);
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
        println!("--- After tick {} ({:.1}s simulated time) ---", tick, tick as f32 * delta);
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
fn run_with_ui() {
    use bevy::prelude::*;
    use bevy::log::LogPlugin;

    println!("Starting Traffic Sim UI...");
    println!();
    println!("Camera Controls:");
    println!("  W/A/S/D     - Move camera");
    println!("  Q/E         - Rotate camera around center");
    println!("  Z/X         - Zoom in/out");
    println!("  Click+Drag  - Orbital rotation");
    println!("  ESC         - Exit");
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
