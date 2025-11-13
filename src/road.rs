use bevy::prelude::*;

/// Component that marks an entity as a road segment
#[derive(Component)]
pub struct Road {
    pub start: Vec3,
    pub end: Vec3,
    pub angle: f32, // Rotation angle in radians (Y-axis rotation)
}

impl Road {
    /// Creates a new road segment between two points
    pub fn new(start: Vec3, end: Vec3) -> Self {
        let direction = (end - start).normalize();
        let angle = direction.x.atan2(direction.z);
        Self { start, end, angle }
    }

    /// Spawns the road entity with all necessary components
    pub fn spawn(
        &self,
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<StandardMaterial>>,
    ) {
        const ROAD_WIDTH: f32 = 0.4;
        const ROAD_HEIGHT: f32 = 0.02;
        let road_color = Color::srgb(0.2, 0.2, 0.2);

        // Calculate road properties
        let length = self.start.distance(self.end);
        let midpoint = (self.start + self.end) / 2.0;
        
        // Use the stored angle for rotation
        let rotation = Quat::from_rotation_y(self.angle);

        // Spawn road segment
        commands.spawn(RoadBundle {
            road: Road {
                start: self.start,
                end: self.end,
                angle: self.angle,
            },
            mesh: Mesh3d(meshes.add(Cuboid::new(ROAD_WIDTH, ROAD_HEIGHT, length))),
            material: MeshMaterial3d(materials.add(road_color)),
            transform: Transform::from_translation(Vec3::new(
                midpoint.x,
                ROAD_HEIGHT / 2.0,
                midpoint.z,
            ))
            .with_rotation(rotation),
        });
    }
}

/// Resource to store all road segments for pathfinding
#[derive(Resource, Default)]
pub struct RoadNetwork {
    pub roads: Vec<(Vec3, Vec3, f32)>, // (start, end, angle)
}

/// Bundle for spawning a road with all necessary components
#[derive(Bundle)]
pub struct RoadBundle {
    pub road: Road,
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
}

/// System to spawn roads connecting houses
pub fn spawn_roads(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut road_network: ResMut<RoadNetwork>,
) {
    // Define house positions (should match house.rs)
    let house_positions = vec![
        Vec3::new(-8.0, 0.0, -8.0),
        Vec3::new(8.0, 0.0, -8.0),
        Vec3::new(-8.0, 0.0, 8.0),
        Vec3::new(8.0, 0.0, 8.0),
    ];

    // Create road connections between houses
    let road_connections = vec![
        (0, 1), // Left to Right (top)
        (2, 3), // Left to Right (bottom)
        (0, 2), // Top to Bottom (left)
        (1, 3), // Top to Bottom (right)
    ];

    for (start_idx, end_idx) in road_connections {
        let start = house_positions[start_idx];
        let end = house_positions[end_idx];
        
        // Create and spawn road using the Road struct
        let road = Road::new(start, end);
        
        // Store in road network for pathfinding
        road_network.roads.push((start, end, road.angle));

        road.spawn(&mut commands, &mut meshes, &mut materials);
    }
}

/// Helper function to spawn a road between two points
pub fn spawn_road(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &mut ResMut<RoadNetwork>,
    start: Vec3,
    end: Vec3,
) {
    // Create and spawn road using the Road struct
    let road = Road::new(start, end);
    
    // Store in road network for pathfinding
    road_network.roads.push((start, end, road.angle));

    road.spawn(commands, meshes, materials);
}

/// Plugin to register all road-related systems
pub struct RoadPlugin;

impl Plugin for RoadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoadNetwork>()
            .add_systems(Startup, spawn_roads);
    }
}
