mod simulation;

#[cfg(feature = "ui")]
mod car;
#[cfg(feature = "ui")]
mod factory;
#[cfg(feature = "ui")]
mod house;
#[cfg(feature = "ui")]
mod interface;
#[cfg(feature = "ui")]
mod intersection;
#[cfg(feature = "ui")]
mod road;
#[cfg(feature = "ui")]
mod road_network;
#[cfg(feature = "ui")]
mod shop;
#[cfg(feature = "ui")]
mod two_way_road;
#[cfg(feature = "ui")]
mod world;

use clap::Parser;

#[derive(Parser)]
#[command(name = "traffic_sim")]
#[command(about = "Traffic simulation with optional UI")]
struct Cli {
    /// Run with the Bevy game engine UI
    #[arg(long)]
    ui: bool,

    /// Number of simulation ticks to run in headless mode
    #[arg(long, default_value = "100")]
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
    println!();

    let mut world = simulation::SimWorld::create_test_world();

    println!("Initial state:");
    world.print_summary();
    println!();

    // Run simulation
    for tick in 1..=ticks {
        world.tick(delta);

        // Print progress every 10 ticks
        if tick % 10 == 0 {
            println!("--- After tick {} ---", tick);
            world.print_summary();
            println!();
        }
    }

    println!("=== Final State ===");
    world.print_summary();
}

#[cfg(feature = "ui")]
fn run_with_ui() {
    use bevy::prelude::*;
    use bevy::log::LogPlugin;
    use car::CarPlugin;
    use factory::FactoryPlugin;
    use house::HousePlugin;
    use interface::InterfacePlugin;
    use intersection::IntersectionPlugin;
    use road::RoadPlugin;
    use shop::ShopPlugin;
    use world::WorldPlugin;

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
        .add_plugins((WorldPlugin, RoadPlugin, IntersectionPlugin, CarPlugin, HousePlugin, FactoryPlugin, ShopPlugin, InterfacePlugin))
        .add_systems(Update, handle_input)
        .run();
}

#[cfg(feature = "ui")]
/// Handle basic keyboard input
fn handle_input(
    keyboard: bevy::prelude::Res<bevy::prelude::ButtonInput<bevy::prelude::KeyCode>>,
    mut exit: bevy::prelude::MessageWriter<bevy::prelude::AppExit>,
) {
    if keyboard.just_pressed(bevy::prelude::KeyCode::Escape) {
        exit.write(bevy::prelude::AppExit::Success);
    }
}
