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

impl House {
    /// Creates a new house with the specified number of occupants
    pub fn new(occupants: u32) -> Self {
        Self { occupants }
    }

    /// Spawns the house entity with all necessary components
    pub fn spawn(
        &self,
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<StandardMaterial>>,
        position: Vec3,
    ) {
        const HOUSE_SIZE: f32 = 1.0;
        let house_color = Color::srgb(0.7, 0.6, 0.4);

        commands.spawn(HouseBundle {
            house: House {
                occupants: self.occupants,
            },
            mesh: Mesh3d(meshes.add(Cuboid::new(HOUSE_SIZE, HOUSE_SIZE, HOUSE_SIZE))),
            material: MeshMaterial3d(materials.add(house_color)),
            transform: Transform::from_translation(position),
        });
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
    // commands: Commands,
    // meshes: ResMut<Assets<Mesh>>,
    // materials: ResMut<Assets<StandardMaterial>>,
) {
    // // Spawn a couple of sample houses (smaller size)
    // let house_positions = vec![
    //     Vec3::new(-8.0, 0.5, -8.0),
    //     Vec3::new(8.0, 0.5, -8.0),
    //     Vec3::new(-8.0, 0.5, 8.0),
    //     Vec3::new(8.0, 0.5, 8.0),
    // ];

    // for position in house_positions {
    //     let house = House::new(2);
    //     house.spawn(&mut commands, &mut meshes, &mut materials, position);
    // }
}

/// Helper function to spawn a house at a given position
pub fn spawn_house(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
) {
    let house = House::new(2);
    house.spawn(commands, meshes, materials, position);
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
