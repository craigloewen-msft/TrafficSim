# TrafficSim

A traffic simulation game built with Bevy, demonstrating cars navigating through a road network between houses.

## Features

- **3D Traffic Simulation**: Watch cars navigate through intersections and roads
- **Headless Mode**: Run simulations without a GUI for testing and validation
- **Statistics Tracking**: Monitor simulation performance with detailed metrics
- **Interactive Mode**: Build roads and manage the simulation interactively (GUI mode)

## Prerequisites

### Linux
```bash
sudo apt-get install -y libwayland-dev libxkbcommon-dev libasound2-dev libudev-dev
```

### Windows
No additional dependencies required.

### macOS
No additional dependencies required.

## Building

```bash
cargo build --release
```

## Running

### Interactive Mode (with GUI)
```bash
cargo run --release
```

Controls:
- **Mouse Click**: Place roads and interact with UI
- **ESC**: Exit the simulation

### Headless Mode (for testing/validation)
```bash
cargo run --release -- --headless
```

The headless mode will:
- Run the simulation for 30 seconds or until all cars complete their routes
- Output detailed statistics to the console
- Exit automatically with a summary of the simulation

#### Example Output
```
=== SIMULATION COMPLETE ===
Elapsed time: 30.00s
Total cars spawned: 42
Total cars completed: 32
Active cars: 10
Total intersections: 20
Total roads: 31
Success rate: 76.2%
```

## Testing

Run all tests:
```bash
cargo test
```

Run specific test:
```bash
cargo test test_headless_simulation_runs
```

## Project Structure

```
src/
├── main.rs           # Application entry point, headless mode support
├── car.rs            # Car entity and movement logic
├── house.rs          # House entities that spawn cars
├── intersection.rs   # Intersection points in the road network
├── road.rs           # Road entities connecting intersections
├── road_network.rs   # Graph structure for pathfinding
├── world.rs          # World setup (camera, lighting, ground)
└── interface.rs      # UI components (GUI mode only)

tests/
└── simulation_test.rs # Integration tests for headless mode
```

## For AI Agents and Copilot

This repository is designed to be AI-friendly:

### Validation
The headless mode allows AI agents to:
- Run the simulation without requiring a display
- Validate that the simulation works correctly
- Check performance metrics and success rates
- Verify that cars navigate properly through the road network

### Testing
Use the automated tests to verify changes:
```bash
# Run all tests
cargo test

# Run just the simulation tests
cargo test --test simulation_test
```

### Making Changes
1. All changes should maintain or improve the success rate (target: >50%)
2. Use `cargo clippy` to check for issues
3. Use `cargo fmt` to format code
4. Run headless mode to validate simulation still works
5. Update tests if adding new features

### Common Tasks

#### Adding a new feature
1. Implement the feature
2. Add tests if applicable
3. Run `cargo test` to ensure nothing breaks
4. Run `cargo run -- --headless` to verify simulation still works
5. Check the success rate remains acceptable

#### Debugging issues
1. Run in headless mode with logging:
   ```bash
   RUST_LOG=traffic_sim=debug cargo run -- --headless
   ```
2. Check the statistics output for anomalies
3. Review the test output for specific failures

## CI/CD

This project uses GitHub Actions for continuous integration:
- **Build**: Ensures the project compiles
- **Test**: Runs all automated tests
- **Fmt**: Checks code formatting
- **Clippy**: Runs linter checks
- **Headless Simulation**: Validates the simulation runs successfully

See `.github/workflows/ci.yml` for details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on contributing to this project.

## License

This project is open source and available for educational purposes.
