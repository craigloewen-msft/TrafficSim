//! World setup systems for camera, lighting, and ground

use bevy::prelude::*;

use super::components::{Ground, MainCamera};

/// System to setup the world environment (ground, lighting, camera)
pub fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn a 3D camera with top-down view
    commands.spawn((
        MainCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 70.0, 0.0).looking_at(Vec3::ZERO, Vec3::Z),
    ));

    // Spawn a directional light
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Spawn a ground plane
    commands.spawn((
        Ground,
        Mesh3d(meshes.add(Plane3d::default().mesh().size(200.0, 200.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));
}
