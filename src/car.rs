use bevy::prelude::*;

/// Component that marks an entity as a car
#[derive(Component)]
pub struct Car {
    pub speed: f32,
    pub max_speed: f32,
}

impl Default for Car {
    fn default() -> Self {
        Self {
            speed: 1.0,
            max_speed: 5.0,
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
) {
    // Spawn a sample car
    commands.spawn(CarBundle {
        car: Car::default(),
        mesh: Mesh3d(meshes.add(Cuboid::new(2.0, 1.0, 1.0))),
        material: MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))),
        transform: Transform::from_xyz(3.0, 0.5, 0.0),
    });
}

/// System to update car movement logic
pub fn update_cars(
    time: Res<Time>,
    mut car_query: Query<(&Car, &mut Transform)>,
) {
    for (car, mut transform) in car_query.iter_mut() {
        // Simple forward movement for now
        transform.translation.z -= car.speed * time.delta_secs();
        
        // Wrap around if car goes too far
        if transform.translation.z < -15.0 {
            transform.translation.z = 15.0;
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
