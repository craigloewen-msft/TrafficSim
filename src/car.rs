use bevy::prelude::*;
use crate::road::RoadNetwork;

/// Component that marks an entity as a car
#[derive(Component)]
pub struct Car {
    pub speed: f32,
    pub max_speed: f32,
    pub current_road_index: usize,
    pub progress: f32, // 0.0 to 1.0 along the current road
}

impl Default for Car {
    fn default() -> Self {
        Self {
            speed: 2.0,
            max_speed: 5.0,
            current_road_index: 0,
            progress: 0.0,
        }
    }
}

/// Bundle for spawning a car with all necessary components
#[derive(Bundle)]
pub struct CarBundle {
    pub car: Car,
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
}

/// System to spawn cars in the world
pub fn spawn_cars(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    road_network: Res<RoadNetwork>,
) {
    // Spawn cars on different roads
    if !road_network.roads.is_empty() {
        let car_data = vec![
            (0, 0.2, Color::srgb(0.8, 0.2, 0.2)),
            (1, 0.5, Color::srgb(0.2, 0.2, 0.8)),
            (2, 0.7, Color::srgb(0.2, 0.8, 0.2)),
        ];

        for (road_idx, progress, color) in car_data {
            if road_idx < road_network.roads.len() {
                let (start, _, _) = road_network.roads[road_idx];
                commands.spawn(CarBundle {
                    car: Car {
                        current_road_index: road_idx,
                        progress,
                        ..default()
                    },
                    mesh: Mesh3d(meshes.add(Cuboid::new(0.3, 0.2, 0.5))),
                    material: MeshMaterial3d(materials.add(color)),
                    transform: Transform::from_translation(start + Vec3::new(0.0, 0.15, 0.0)),
                });
            }
        }
    }
}

/// System to update car movement logic
pub fn update_cars(
    time: Res<Time>,
    road_network: Res<RoadNetwork>,
    mut car_query: Query<(&mut Car, &mut Transform)>,
) {
    for (mut car, mut transform) in car_query.iter_mut() {
        if road_network.roads.is_empty() {
            continue;
        }

        // Get current road
        let road_index = car.current_road_index % road_network.roads.len();
        let (start, end, angle) = road_network.roads[road_index];
        
        // Calculate road length
        let road_length = start.distance(end);
        if road_length < 0.01 {
            continue; // Skip zero-length roads
        }

        // Update progress along the road
        car.progress += (car.speed / road_length) * time.delta_secs();

        // If we've reached the end of the road, move to next road
        if car.progress >= 1.0 {
            car.progress = 0.0;
            car.current_road_index = (car.current_road_index + 1) % road_network.roads.len();
            let (new_start, _new_end, new_angle) = road_network.roads[car.current_road_index];
            
            // Update to new road's starting position using the road's angle
            let rotation = Quat::from_rotation_y(new_angle);
            
            transform.translation = new_start + Vec3::new(0.0, 0.15, 0.0);
            transform.rotation = rotation;
        } else {
            // Interpolate position along current road
            let current_pos = start.lerp(end, car.progress);
            
            // Use the stored angle from the road for rotation
            let rotation = Quat::from_rotation_y(angle);

            transform.translation = current_pos + Vec3::new(0.0, 0.15, 0.0);
            transform.rotation = rotation;
        }
    }
}

/// Plugin to register all car-related systems
pub struct CarPlugin;

impl Plugin for CarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_cars)
            .add_systems(Update, update_cars);
    }
}
