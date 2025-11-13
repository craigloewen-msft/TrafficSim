use bevy::prelude::*;

/// Component to mark the ground plane
#[derive(Component)]
pub struct Ground;

/// System to setup the world environment (ground, lighting, camera)
pub fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn a 3D camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 10.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
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
        Mesh3d(meshes.add(Plane3d::default().mesh().size(30.0, 30.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));
}

/// Plugin to register all world-related systems
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_world)
    }
}
