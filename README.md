# Traffic Management Game 🚗🏭

A fun traffic and logistics management game built with Rust and Bevy! Build roads, place buildings, and create efficient delivery networks to reach your goals.

## 🎮 Game Overview

You start with a budget of **$2000** and a basic road network. Your goal is to build an efficient traffic system to complete deliveries and earn money.

### Win Conditions
Achieve **either** of these goals to win:
- Complete **50 shop deliveries** 
- Accumulate **$5000** in cash

### Lose Condition
- Go bankrupt (negative money)

## 💰 Economics

### Building Costs
- **Road**: $50
- **House**: $200
- **Factory**: $500
- **Shop**: $300

### Revenue
- **Worker Trip**: $10 (when a worker completes their shift and returns home)
- **Shop Delivery**: $50 (when a truck delivers goods to a shop and returns)

## 🏗️ Buildings

### Houses 🏠
- Source of workers
- Send cars to factories when labor is needed
- Workers return home after their shift

### Factories 🏭
- Employ workers from houses
- Produce goods when workers complete their shifts
- Send delivery trucks to shops when products are ready
- Can only accept workers when the truck is home (not out making deliveries)

### Shops 🏪
- Receive deliveries from factories
- Generate product demand over time
- Each delivery reduces demand

## 🕹️ Controls

### Camera
- **W/A/S/D**: Move camera
- **Q/E**: Rotate camera around center
- **Z/X**: Zoom in/out
- **Mouse Drag**: Orbital rotation
- **ESC**: Exit

### Building
- **1** or **Road Button**: Road mode (click two points to create a road)
- **2** or **House Button**: House mode (click to place)
- **3** or **Factory Button**: Factory mode (click to place)
- **4** or **Shop Button**: Shop mode (click to place)

### Smart Placement
Buildings automatically snap to:
- Nearby intersections
- Existing roads (splitting them to create a new intersection)
- Or create a new intersection at the clicked position

## 🚀 Running the Game

### Play the Game (UI Mode)
```bash
cargo run --features ui -- --ui
```

### Run Test Simulation (Headless Mode)
```bash
cargo run --no-default-features
# or with custom parameters:
cargo run --no-default-features -- --ticks 1000 --delta 0.1
```

### Run Tests
```bash
cargo test --no-default-features
```

## 💡 Strategy Tips

1. **Build Efficiently**: Shorter roads mean faster deliveries and quicker profits
2. **Balance Your Network**: You need houses for workers, factories for production, and shops for deliveries
3. **Watch Your Budget**: Plan your builds carefully - going bankrupt means game over
4. **Optimize Routes**: Strategic road placement can drastically improve delivery times
5. **Start Small**: Build a few buildings first, earn money from deliveries, then expand
6. **Factory Availability**: Factories can only accept new workers when their truck is home, so plan accordingly

## 🎯 Game Mechanics

The simulation runs automatically once buildings are placed:
- Houses automatically send workers to factories with high labor demand
- Workers spend time at factories, then return home
- Factories automatically dispatch trucks to shops with high product demand
- Trucks deliver goods and return to factories

Monitor the **Global Demand** indicators to see where bottlenecks are forming!

## 🏆 Scoring

Your progress is tracked in the top-left corner:
- **Money**: Your current budget
- **Worker Trips**: Total completed worker round trips
- **Shop Deliveries**: Progress toward the 50 delivery goal
- **Goal Status**: Current objective and win/lose status

## 📝 Development

Built with:
- **Rust** - Systems programming language
- **Bevy** - Data-driven game engine
- **Petgraph** - Graph algorithms for pathfinding

The game features a clean separation between:
- **Simulation** (`src/simulation/`) - Game logic, traffic simulation, pathfinding
- **UI** (`src/ui/`) - Bevy-based 3D visualization and user interface

## License

MIT License - See LICENSE file for details
