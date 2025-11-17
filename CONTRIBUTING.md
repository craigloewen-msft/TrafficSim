# Contributing to TrafficSim

Thank you for your interest in contributing to TrafficSim! This guide will help both human developers and AI agents understand how to contribute effectively.

## For AI Agents (like GitHub Copilot)

### Understanding the Project

This is a traffic simulation built with the Bevy game engine. The simulation:
- Creates a road network with intersections
- Spawns cars that navigate between houses
- Can run in headless mode for testing and validation

### Key Capabilities

#### Running the Simulation
You can validate your changes by running:
```bash
# Headless mode (no GUI, outputs statistics)
cargo run -- --headless

# Interactive mode (requires display)
cargo run
```

#### Verifying Changes
```bash
# Run tests
cargo test

# Check code style
cargo fmt --check
cargo clippy -- -D warnings
```

### Making Changes

#### 1. Understanding Success Criteria
The simulation tracks these metrics:
- **Total cars spawned**: Number of cars created
- **Total cars completed**: Cars that reached their destination
- **Success rate**: (completed / spawned) × 100
- **Active cars**: Currently running cars

A healthy simulation should have:
- Success rate > 50%
- Cars should spawn and complete routes
- No panics or crashes

#### 2. Testing Your Changes

After making changes, always:

1. **Build the project**
   ```bash
   cargo build
   ```

2. **Run tests**
   ```bash
   cargo test
   ```

3. **Run headless simulation**
   ```bash
   cargo run -- --headless
   ```

4. **Check the output**
   Look for:
   - `SIMULATION COMPLETE` message
   - Reasonable success rate (>50%)
   - No error messages or panics

#### 3. Common Development Tasks

##### Adding a New Feature
1. Identify which module(s) need changes (car, road, house, etc.)
2. Make minimal, focused changes
3. Update or add tests if needed
4. Run the full test suite
5. Verify in headless mode

##### Fixing a Bug
1. Write a test that reproduces the bug (if possible)
2. Fix the bug
3. Verify the test passes
4. Run all tests to ensure no regression
5. Check headless mode works correctly

##### Performance Optimization
1. Run headless mode to get baseline metrics
2. Make your optimization
3. Run headless mode again and compare
4. Ensure success rate hasn't decreased
5. Document the improvement

#### 4. Code Quality

Always run before committing:
```bash
# Format code
cargo fmt

# Check for common mistakes
cargo clippy

# Run tests
cargo test
```

### Understanding the Architecture

```
┌─────────────┐
│    main     │ ← Entry point, handles headless/GUI mode
└──────┬──────┘
       │
       ├─────────┐
       │         │
┌──────▼──────┐ ┌▼────────────┐
│   world     │ │  interface  │ ← GUI only
│ (camera,    │ │  (UI)       │
│  lighting)  │ └─────────────┘
└─────────────┘
       │
┌──────▼───────────────────┐
│    road_network          │ ← Graph for pathfinding
│  (intersections + roads) │
└──────┬───────────────────┘
       │
   ┌───┴───┬────────┬────────┐
   │       │        │        │
┌──▼──┐ ┌─▼────┐ ┌─▼───┐ ┌──▼──┐
│ car │ │ road │ │house│ │inter│
└─────┘ └──────┘ └─────┘ └─────┘
```

### Important Files

- **src/main.rs**: Application setup, headless mode logic, statistics
- **src/car.rs**: Car spawning and movement
- **src/road_network.rs**: Pathfinding and graph structure
- **tests/simulation_test.rs**: Automated tests

### Debugging Tips

1. **Enable debug logging**
   ```bash
   RUST_LOG=traffic_sim=debug cargo run -- --headless
   ```

2. **Check specific module**
   ```bash
   RUST_LOG=traffic_sim::car=trace cargo run -- --headless
   ```

3. **Run a specific test**
   ```bash
   cargo test test_headless_simulation_runs -- --nocapture
   ```

## For Human Developers

### Setup

1. Clone the repository
2. Install Rust (https://rustup.rs/)
3. Install system dependencies (see README.md)
4. Build: `cargo build`

### Development Workflow

1. Create a new branch for your feature/fix
2. Make your changes
3. Test thoroughly (see "Testing Your Changes" above)
4. Format and lint your code
5. Submit a pull request

### Pull Request Guidelines

- Provide a clear description of the changes
- Include test results from headless mode
- Ensure all CI checks pass
- Update documentation if needed

### Code Style

- Follow Rust conventions
- Use `cargo fmt` for formatting
- Address `cargo clippy` warnings
- Add comments for complex logic

### Testing Philosophy

- Unit tests for individual components
- Integration tests for the full simulation
- Headless mode as a validation tool
- Aim for >50% success rate in simulations

## Questions?

For questions or issues:
1. Check the README.md first
2. Look at existing issues on GitHub
3. Create a new issue with:
   - Clear description
   - Steps to reproduce (if bug)
   - Output from headless mode
   - System information

## License

By contributing, you agree that your contributions will be licensed under the same license as the project.
