use std::process::Command;

/// Test that the simulation runs in headless mode without crashing
#[test]
fn test_headless_simulation_runs() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--headless"])
        .env("RUST_LOG", "warn,traffic_sim=info")
        .output()
        .expect("Failed to execute simulation");

    // Check that the simulation exited successfully
    assert!(
        output.status.success(),
        "Simulation failed to run in headless mode. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify simulation complete message is present
    assert!(
        stderr.contains("SIMULATION COMPLETE"),
        "Simulation did not complete properly. stderr: {}",
        stderr
    );
}

/// Test that simulation statistics are logged
#[test]
fn test_simulation_statistics_logged() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--headless"])
        .env("RUST_LOG", "warn,traffic_sim=info")
        .output()
        .expect("Failed to execute simulation");

    assert!(output.status.success(), "Simulation failed to run");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check for key statistics in the output
    assert!(
        stderr.contains("Total cars spawned:"),
        "Missing 'Total cars spawned' statistic"
    );
    assert!(
        stderr.contains("Total cars completed:"),
        "Missing 'Total cars completed' statistic"
    );
    assert!(
        stderr.contains("Active cars:"),
        "Missing 'Active cars' statistic"
    );
    assert!(
        stderr.contains("Total intersections:"),
        "Missing 'Total intersections' statistic"
    );
    assert!(
        stderr.contains("Total roads:"),
        "Missing 'Total roads' statistic"
    );
    assert!(
        stderr.contains("Success rate:"),
        "Missing 'Success rate' statistic"
    );
}

/// Test that cars are spawned during simulation
#[test]
fn test_cars_spawn_during_simulation() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--headless"])
        .env("RUST_LOG", "warn,traffic_sim=info")
        .output()
        .expect("Failed to execute simulation");

    assert!(output.status.success(), "Simulation failed to run");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check that cars were spawned
    assert!(
        stderr.contains("SPAWNING CARS"),
        "No cars spawning message found"
    );

    // Extract the total cars spawned count
    let spawned_line = stderr
        .lines()
        .find(|line| line.contains("Total cars spawned:"))
        .expect("Could not find 'Total cars spawned' line");

    // Parse the number - handle log format with timestamp
    let parts: Vec<&str> = spawned_line.split("Total cars spawned:").collect();
    let spawned_count: u32 = parts
        .get(1)
        .and_then(|s| s.trim().parse().ok())
        .expect("Could not parse spawned count");

    assert!(spawned_count > 0, "No cars were spawned during simulation");
}

/// Test that the simulation has a reasonable success rate
#[test]
fn test_simulation_success_rate() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--headless"])
        .env("RUST_LOG", "warn,traffic_sim=info")
        .output()
        .expect("Failed to execute simulation");

    assert!(output.status.success(), "Simulation failed to run");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Extract the success rate
    let success_rate_line = stderr
        .lines()
        .find(|line| line.contains("Success rate:"))
        .expect("Could not find 'Success rate' line");

    // Parse the percentage - handle log format with timestamp
    // Format: "2025-11-17T17:10:52.598815Z  INFO traffic_sim: Success rate: 76.2%"
    let parts: Vec<&str> = success_rate_line.split("Success rate:").collect();
    let rate_str = parts
        .get(1)
        .and_then(|s| s.trim().strip_suffix('%'))
        .unwrap_or_else(|| {
            panic!(
                "Could not parse success rate from line: {}",
                success_rate_line
            )
        });

    let success_rate: f32 = rate_str
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("Could not parse '{}' as float", rate_str));

    // Success rate should be reasonable (at least 50%)
    assert!(
        success_rate >= 50.0,
        "Success rate too low: {}%",
        success_rate
    );
}
