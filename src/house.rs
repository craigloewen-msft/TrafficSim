use bevy::prelude::*;

use crate::intersection;
use crate::intersection::IntersectionEntity;

/// Component that marks an entity as a house
#[derive(Component)]
pub struct House {
    pub spawn_rate: u32,
    pub intersection_entity: intersection::IntersectionEntity,
}

impl House {}

pub fn spawn_house(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    intersection_entity: intersection::IntersectionEntity,
    position: Vec3,
) {
    const HOUSE_SIZE: f32 = 1.0;
    let house_color = Color::srgb(0.7, 0.6, 0.4);

    commands.spawn((
        House {
            spawn_rate: 2,
            intersection_entity,
        },
        Mesh3d(meshes.add(Cuboid::new(HOUSE_SIZE, HOUSE_SIZE, HOUSE_SIZE))),
        MeshMaterial3d(materials.add(house_color)),
        Transform::from_translation(position),
    ));
}

/// System to update house logic (placeholder for now)
pub fn update_houses(_time: Res<Time>, _house_query: Query<(&House, &Transform)>) {
    // Future: Update house logic here
    // For example: spawn cars, manage occupants, etc.
}

/// Plugin to register all house-related systems
pub struct HousePlugin;

impl Plugin for HousePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_houses);
    }
}
