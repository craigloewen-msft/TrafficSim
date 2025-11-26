# GitHub Copilot Instructions for TrafficSim

## Project Overview
TrafficSim is a Rust-based traffic simulation built with the Bevy game engine.
## Architecture & Design Patterns

### ECS Pattern (Entity Component System)
- This project uses Bevy's ECS architecture extensively
- Entities are wrapped in type-safe wrappers: `CarEntity`, `RoadEntity`, `IntersectionEntity`
- Each major concept has its own plugin: `WorldPlugin`, `RoadPlugin`, `IntersectionPlugin`, `CarPlugin`, `HousePlugin`, `FactoryPlugin`, `InterfacePlugin`
- Systems operate on components through queries

### Error Handling
- Use `anyhow::Result` for functions that can fail
- Provide context with `.context("Description")` when propagating errors
- Log errors appropriately using `bevy::log::{error, warn, info, debug}`

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
8. There should only ever be one source of truth or function - do not duplicate logic. E.g: There is only one 'spawn_car' function, even if cars are spawned from multiple places.