use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::house::spawn_house;
use crate::road::spawn_road_at_positions;
use crate::road_network::RoadNetwork;

/// Resource to track what the player wants to spawn
#[derive(Resource, Default, PartialEq, Clone)]
pub enum SpawnMode {
    #[default]
    None,
    House,
    Road,
}

/// Resource to track the first point when spawning a road
#[derive(Resource, Default)]
pub struct RoadSpawnState {
    pub first_point: Option<Vec3>,
}

/// Marker component for UI buttons
#[derive(Component)]
pub struct SpawnButton {
    pub mode: SpawnMode,
}

/// Marker for the toolbar container
#[derive(Component)]
struct Toolbar;

/// System to setup the UI toolbar
pub fn setup_ui(mut commands: Commands) {
    // Create the toolbar at the bottom of the screen
    commands
        .spawn((
            Toolbar,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(60.0),
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 0.9)),
        ))
        .with_children(|parent| {
            // House spawn button
            parent
                .spawn((
                    SpawnButton {
                        mode: SpawnMode::House,
                    },
                    Button,
                    Node {
                        width: Val::Px(120.0),
                        height: Val::Px(40.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.4, 0.4, 0.6)),
                ))
                .with_child((
                    Text::new("Spawn House"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));

            // Road spawn button
            parent
                .spawn((
                    SpawnButton {
                        mode: SpawnMode::Road,
                    },
                    Button,
                    Node {
                        width: Val::Px(120.0),
                        height: Val::Px(40.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                ))
                .with_child((
                    Text::new("Spawn Road"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));

            // Clear/Cancel button
            parent
                .spawn((
                    SpawnButton {
                        mode: SpawnMode::None,
                    },
                    Button,
                    Node {
                        width: Val::Px(120.0),
                        height: Val::Px(40.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.5, 0.2, 0.2)),
                ))
                .with_child((
                    Text::new("Cancel"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
        });
}

/// System to handle button interactions
pub fn handle_button_interaction(
    mut spawn_mode: ResMut<SpawnMode>,
    mut road_state: ResMut<RoadSpawnState>,
    mut interaction_query: Query<
        (&Interaction, &SpawnButton, &mut BackgroundColor),
        Changed<Interaction>,
    >,
) {
    for (interaction, button, mut bg_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *spawn_mode = button.mode.clone();
                // Reset road spawn state when changing modes
                road_state.first_point = None;
                
                // Update button colors to show selection
                *bg_color = match button.mode {
                    SpawnMode::House => Color::srgb(0.2, 0.7, 0.3).into(),
                    SpawnMode::Road => Color::srgb(0.1, 0.1, 0.1).into(),
                    SpawnMode::None => Color::srgb(0.7, 0.2, 0.2).into(),
                };
            }
            Interaction::Hovered => {
                *bg_color = match button.mode {
                    SpawnMode::House => Color::srgb(0.5, 0.5, 0.7).into(),
                    SpawnMode::Road => Color::srgb(0.4, 0.4, 0.4).into(),
                    SpawnMode::None => Color::srgb(0.6, 0.3, 0.3).into(),
                };
            }
            Interaction::None => {
                *bg_color = match button.mode {
                    SpawnMode::House => Color::srgb(0.4, 0.4, 0.6).into(),
                    SpawnMode::Road => Color::srgb(0.3, 0.3, 0.3).into(),
                    SpawnMode::None => Color::srgb(0.5, 0.2, 0.2).into(),
                };
            }
        }
    }
}

/// System to update button colors based on current spawn mode
pub fn update_button_colors(
    spawn_mode: Res<SpawnMode>,
    mut button_query: Query<(&SpawnButton, &mut BackgroundColor)>,
) {
    if !spawn_mode.is_changed() {
        return;
    }

    for (button, mut bg_color) in &mut button_query {
        // Highlight the active mode
        if button.mode == *spawn_mode {
            *bg_color = match button.mode {
                SpawnMode::House => Color::srgb(0.2, 0.7, 0.3).into(),
                SpawnMode::Road => Color::srgb(0.1, 0.1, 0.1).into(),
                SpawnMode::None => Color::srgb(0.3, 0.3, 0.3).into(),
            };
        } else {
            *bg_color = match button.mode {
                SpawnMode::House => Color::srgb(0.4, 0.4, 0.6).into(),
                SpawnMode::Road => Color::srgb(0.3, 0.3, 0.3).into(),
                SpawnMode::None => Color::srgb(0.5, 0.2, 0.2).into(),
            };
        }
    }
}

/// System to handle mouse clicks and spawn entities
pub fn handle_world_clicks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut road_network: ResMut<RoadNetwork>,
    spawn_mode: Res<SpawnMode>,
    mut road_state: ResMut<RoadSpawnState>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
) {
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = window_query.single() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    // Cast a ray from the camera through the cursor position
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    // Calculate intersection with the ground plane (y = 0)
    // Ray equation: point = origin + t * direction
    // For y = 0: origin.y + t * direction.y = 0
    // Therefore: t = -origin.y / direction.y
    let t = -ray.origin.y / ray.direction.y;
    
    if t < 0.0 {
        return; // Ray is pointing away from the ground
    }

    let world_position = ray.origin + ray.direction * t;
    let spawn_position = Vec3::new(world_position.x, 0.5, world_position.z);

    match *spawn_mode {
        SpawnMode::House => {
            spawn_house(
                &mut commands,
                &mut meshes,
                &mut materials,
                spawn_position,
            );
        }
        SpawnMode::Road => {
            if let Some(first_point) = road_state.first_point {
                // We have a first point, so spawn the road
                spawn_road_at_positions(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut road_network,
                    first_point,
                    spawn_position,
                );
                road_state.first_point = None;
            } else {
                // This is the first point
                road_state.first_point = Some(spawn_position);
            }
        }
        SpawnMode::None => {}
    }
}

/// Plugin to register all interface-related systems
pub struct InterfacePlugin;

impl Plugin for InterfacePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnMode>()
            .init_resource::<RoadSpawnState>()
            .add_systems(Startup, setup_ui)
            .add_systems(
                Update,
                (
                    handle_button_interaction,
                    update_button_colors,
                    handle_world_clicks,
                ),
            );
    }
}
