# GitHub Copilot Instructions for TrafficSim

## Project Overview
TrafficSim is a Rust-based traffic simulation built with the Bevy game engine.

## Building and Testing
- Build: `cargo build` (or `cargo build --no-default-features` for headless mode without UI dependencies)
- Run: `cargo run` (UI mode) or `cargo run -- --ticks 100` (headless mode)
- Test: `cargo test` (or `cargo test --no-default-features` for headless testing)
- Lint: `cargo clippy`
- Format: `cargo fmt`

## Architecture & Design Patterns

### ECS Pattern (Entity Component System)
- This project uses Bevy's ECS architecture extensively
- Entities are wrapped in type-safe wrappers: `CarEntity`, `RoadEntity`, `IntersectionEntity`
- Each major concept has its own plugin: `WorldPlugin`, `RoadPlugin`, `IntersectionPlugin`, `CarPlugin`, `HousePlugin`, `FactoryPlugin`, `InterfacePlugin`
- Systems operate on components through queries

### Error Handling (IMPORTANT)
- **Always** use `anyhow::Result` for functions that can fail
- **Always** propagate errors using the `?` operator with `.context("Description")` to add meaningful context
- Never silently ignore errors - either handle them explicitly or propagate them
- Log errors appropriately using `bevy::log::{error, warn, info, debug}`

Example (note: `Result` here is `anyhow::Result` via the import):
```rust
use anyhow::{Context, Result};

pub fn spawn_car(&mut self, from: IntersectionId, to: IntersectionId) -> Result<CarId> {
    let path = self.road_network
        .find_path(from, to)
        .context("No path found to destination")?;
    // ...
}
```

## Module Structure

## Coding Standards

### Naming Conventions
- Use `snake_case` for variables, functions, and modules
- Use `PascalCase` for types, structs, enums, and traits
- Constants use `SCREAMING_SNAKE_CASE`

### Component Design
Components should be focused and composable:
```rust
#[derive(Component, Debug)]
pub struct Intersection {
    pub occupied_by: Option<CarEntity>,
    pub occupation_timer: f32,
    pub crossing_time: f32,
}
```

### Resource Management
- Use Bevy's `Resource` for global state like `RoadNetwork`
- Query parameters should use `Res` for read-only and `ResMut` for mutable access


## Common Patterns

## Testing & Debugging
- Use `bevy::log` macros for debugging: `debug!()`, `info!()`, `warn!()`, `error!()`
- Log filter is configured in main.rs: `"warn,traffic_sim=debug"`
- Run with `cargo run -- --test` to do a test

## When Adding New Features
1. Consider if it needs a new Component, Resource, or System
2. Create a dedicated plugin if it's a major feature area
3. Use type-safe entity wrappers for new entity types
4. Add appropriate error handling with `anyhow::Result`
5. Include debug logging at appropriate levels
6. Update relevant queries to include/exclude new components as needed
7. Follow the existing module structure and plugin organization

### Single Source of Truth (IMPORTANT)
**Each functionality should exist in exactly one place.** Do not duplicate logic across the codebase.

Example: The `spawn_car` function exists only in `car.rs` (simulation module). Even though cars may be spawned from multiple places (houses, factories), they all call the same single `spawn_car` function.

When adding new functionality:
- Check if similar functionality already exists before creating new functions
- If a function needs to be called from multiple places, call the existing function rather than duplicating code
- Keep related logic together in the appropriate module