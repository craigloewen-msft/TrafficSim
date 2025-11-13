use bevy::prelude::*;

/// Component that marks an entity as a house
#[derive(Component)]
pub struct House {
    pub occupants: u32,
}

impl Default for House {
    fn default() -> Self {
        Self {
            occupants: 0,
        }
    }
}

/// Bundle for spawning a house with all necessary components
#[derive(Bundle)]
pub struct HouseBundle {
    pub house: House,
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
}

/// System to spawn houses in the world
pub fn spawn_houses(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn a couple of sample houses
    let house_positions = vec![
        Vec3::new(-5.0, 1.0, -5.0),
        Vec3::new(5.0, 1.0, -5.0),
        Vec3::new(-5.0, 1.0, 5.0),
    ];

    for position in house_positions {
        commands.spawn(HouseBundle {
            house: House { occupants: 2 },
            mesh: Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 2.0))),
            material: MeshMaterial3d(materials.add(Color::srgb(0.7, 0.6, 0.4))),
            transform: Transform::from_translation(position),
        });
    }
}

/// System to update house logic (placeholder for now)
pub fn update_houses(
    _time: Res<Time>,
    _house_query: Query<(&House, &Transform)>,
) {
    // Future: Update house logic here
    // For example: spawn cars, manage occupants, etc.
}

/// Plugin to register all house-related systems
pub struct HousePlugin;

impl Plugin for HousePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_houses)
            .add_systems(Update, update_houses);
    }
}
