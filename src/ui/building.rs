//! Building mode systems for dynamically adding roads and buildings

use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;

use super::components::{
    BuildModeButton, BuildingMode, BuildingState, EntityMappings, GhostPreview, MainCamera,
    SimWorldResource,
};
use super::spawner::{
    spawn_factory_visual, spawn_house_visual, spawn_intersection_visual, spawn_road_visual,
    spawn_shop_visual,
};
use crate::simulation::Position;
use crate::ui::components::GlobalDemandText;

/// System to setup the building mode UI
pub fn setup_building_ui(mut commands: Commands) {
    // Create game stats toolbar at top-left of screen
    commands
        .spawn((Node {
            width: Val::Auto,
            height: Val::Auto,
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            padding: UiRect::all(Val::Px(10.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(5.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        ))
        .with_children(|parent| {
            // Money display
            parent.spawn((
                Text::new("Money: $0"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.2, 1.0, 0.2)),
                GlobalDemandText::Money,
            ));
            
            // Worker trips
            parent.spawn((
                Text::new("Worker Trips: 0"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                GlobalDemandText::WorkerTrips,
            ));
            
            // Shop deliveries
            parent.spawn((
                Text::new("Shop Deliveries: 0 / 50"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                GlobalDemandText::ShopDeliveries,
            ));
            
            // Goal status
            parent.spawn((
                Text::new("Goal: Deliver 50 shipments!"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 0.5)),
                GlobalDemandText::GoalStatus,
            ));
        });

    // Create global demand toolbar at top of screen (centered)
    commands
        .spawn((Node {
            width: Val::Percent(100.0),
            height: Val::Auto,
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: Val::Px(20.0),
            ..default()
        },))
        .with_children(|parent| {
            // Global Demand label
            parent.spawn((
                Text::new("Building Status:"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            // Factories with trucks out
            spawn_demand_text(
                parent,
                GlobalDemandText::FactoriesWaiting,
                "Factories Busy: 0/0",
                Color::srgb(0.5, 0.5, 0.7),
            );

            // Shops (passive receivers)
            spawn_demand_text(
                parent,
                GlobalDemandText::ShopsWaiting,
                "Shops: 0",
                Color::srgb(0.8, 0.4, 0.6),
            );

            // Houses with cars out
            spawn_demand_text(
                parent,
                GlobalDemandText::HousesWaiting,
                "Houses Busy: 0/0",
                Color::srgb(0.7, 0.6, 0.4),
            );
        });

    // Create UI container at bottom of screen
    commands
        .spawn((Node {
            width: Val::Percent(100.0),
            height: Val::Auto,
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            ..default()
        },))
        .with_children(|parent| {
            // Road button
            spawn_build_button(
                parent,
                BuildingMode::Road,
                "Road [1] - $50",
                Color::srgb(0.3, 0.3, 0.3),
            );
            // House button
            spawn_build_button(
                parent,
                BuildingMode::House,
                "House [2] - $200",
                Color::srgb(0.7, 0.6, 0.4),
            );
            // Factory button
            spawn_build_button(
                parent,
                BuildingMode::Factory,
                "Factory [3] - $500",
                Color::srgb(0.5, 0.5, 0.7),
            );
            // Shop button
            spawn_build_button(
                parent,
                BuildingMode::Shop,
                "Shop [4] - $300",
                Color::srgb(0.8, 0.4, 0.6),
            );
        });
}

fn spawn_demand_text(
    parent: &mut ChildSpawnerCommands,
    demand_type: GlobalDemandText,
    text: &str,
    color: Color,
) {
    parent.spawn((
        demand_type,
        Text::new(text),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(color),
    ));
}

fn spawn_build_button(
    parent: &mut ChildSpawnerCommands,
    mode: BuildingMode,
    text: &str,
    color: Color,
) {
    parent
        .spawn((
            BuildModeButton(mode),
            Button,
            Node {
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor::all(Color::WHITE),
            BackgroundColor(color),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(text),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// System to handle building mode button clicks
pub fn handle_build_buttons(
    mut building_state: ResMut<BuildingState>,
    mut interaction_query: Query<
        (
            &Interaction,
            &BuildModeButton,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Changed<Interaction>,
    >,
) {
    for (interaction, button, mut bg_color, mut border_color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                // Toggle the mode
                if building_state.mode == button.0 {
                    building_state.mode = BuildingMode::None;
                    building_state.road_start = None;
                } else {
                    building_state.mode = button.0;
                    building_state.road_start = None;
                }
            }
            Interaction::Hovered => {
                *border_color = BorderColor::all(Color::srgb(1.0, 1.0, 0.0));
            }
            Interaction::None => {
                *border_color = BorderColor::all(if building_state.mode == button.0 {
                    Color::srgb(0.0, 1.0, 0.0)
                } else {
                    Color::WHITE
                });
            }
        }

        // Update background to show selected state
        let base_color = match button.0 {
            BuildingMode::Road => Color::srgb(0.3, 0.3, 0.3),
            BuildingMode::House => Color::srgb(0.7, 0.6, 0.4),
            BuildingMode::Factory => Color::srgb(0.5, 0.5, 0.7),
            BuildingMode::Shop => Color::srgb(0.8, 0.4, 0.6),
            BuildingMode::None => Color::srgb(0.5, 0.5, 0.5),
        };

        if building_state.mode == button.0 {
            // Brighten when selected (clamp to prevent overflow)
            bg_color.0 = Color::srgba(
                (base_color.to_srgba().red * 1.3).min(1.0),
                (base_color.to_srgba().green * 1.3).min(1.0),
                (base_color.to_srgba().blue * 1.3).min(1.0),
                1.0,
            );
        } else {
            bg_color.0 = base_color;
        }
    }
}

/// System to handle keyboard shortcuts for building modes
pub fn handle_build_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut building_state: ResMut<BuildingState>,
) {
    if keyboard.just_pressed(KeyCode::Digit1) {
        building_state.mode = if building_state.mode == BuildingMode::Road {
            BuildingMode::None
        } else {
            BuildingMode::Road
        };
        building_state.road_start = None;
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        building_state.mode = if building_state.mode == BuildingMode::House {
            BuildingMode::None
        } else {
            BuildingMode::House
        };
        building_state.road_start = None;
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        building_state.mode = if building_state.mode == BuildingMode::Factory {
            BuildingMode::None
        } else {
            BuildingMode::Factory
        };
        building_state.road_start = None;
    }
    if keyboard.just_pressed(KeyCode::Digit4) {
        building_state.mode = if building_state.mode == BuildingMode::Shop {
            BuildingMode::None
        } else {
            BuildingMode::Shop
        };
        building_state.road_start = None;
    }
}

/// System to update cursor position on ground plane
pub fn update_cursor_position(
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut building_state: ResMut<BuildingState>,
    sim_world: Res<SimWorldResource>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        building_state.cursor_position = None;
        building_state.snapped_position = None;
        return;
    };

    // Cast ray from camera through cursor position
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    // Find intersection with ground plane (y = 0)
    let Some(distance) = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y)) else {
        return;
    };

    let world_position = ray.get_point(distance);
    let position = Position::new(world_position.x, 0.0, world_position.z);

    building_state.cursor_position = Some(position);

    // Check for snapping
    let snap_distance = building_state.snap_distance;

    // First check for nearby intersection
    if let Some(closest_intersection) = sim_world
        .0
        .road_network
        .find_closest_intersection(&position)
    {
        if let Some(intersection_pos) = sim_world
            .0
            .road_network
            .get_intersection_position(closest_intersection)
        {
            if position.distance(intersection_pos) <= snap_distance {
                building_state.snapped_position = Some(*intersection_pos);
                return;
            }
        }
    }

    // Then check for nearby road (for splitting)
    if let Some((_, closest_point, _, _)) = sim_world
        .0
        .road_network
        .find_closest_point_on_road(&position)
    {
        if position.distance(&closest_point) <= snap_distance {
            building_state.snapped_position = Some(closest_point);
            return;
        }
    }

    building_state.snapped_position = None;
}

/// System to update ghost preview entities
pub fn update_ghost_preview(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    building_state: Res<BuildingState>,
    ghost_query: Query<Entity, With<GhostPreview>>,
) {
    // Remove old ghost entities
    for entity in ghost_query.iter() {
        commands.entity(entity).despawn();
    }

    // Only show preview if in a building mode
    if building_state.mode == BuildingMode::None {
        return;
    }

    let position = building_state
        .snapped_position
        .or(building_state.cursor_position);

    let Some(pos) = position else {
        return;
    };

    let ghost_color = Color::srgba(1.0, 1.0, 1.0, 0.5);

    match building_state.mode {
        BuildingMode::Road => {
            // Show intersection preview at current position
            commands.spawn((
                GhostPreview,
                Mesh3d(meshes.add(Sphere::new(0.3))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: ghost_color,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                })),
                Transform::from_translation(Vec3::new(pos.x, 0.3, pos.z)),
            ));

            // If we have a road start point, show the road preview
            if let Some(start) = building_state.road_start {
                let length = start.distance(&pos);
                if length > 0.1 {
                    let midpoint =
                        Position::new((start.x + pos.x) / 2.0, 0.0, (start.z + pos.z) / 2.0);
                    let angle = start.angle_to(&pos);

                    commands.spawn((
                        GhostPreview,
                        Mesh3d(meshes.add(Cuboid::new(0.6, 0.02, length))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: ghost_color,
                            alpha_mode: AlphaMode::Blend,
                            ..default()
                        })),
                        Transform::from_translation(Vec3::new(midpoint.x, 0.01, midpoint.z))
                            .with_rotation(Quat::from_rotation_y(angle)),
                    ));

                    // Show start point
                    commands.spawn((
                        GhostPreview,
                        Mesh3d(meshes.add(Sphere::new(0.3))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgba(0.0, 1.0, 0.0, 0.7),
                            alpha_mode: AlphaMode::Blend,
                            ..default()
                        })),
                        Transform::from_translation(Vec3::new(start.x, 0.3, start.z)),
                    ));
                }
            }
        }
        BuildingMode::House => {
            commands.spawn((
                GhostPreview,
                Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba(0.7, 0.6, 0.4, 0.5),
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                })),
                Transform::from_translation(Vec3::new(pos.x, 0.5, pos.z)),
            ));
        }
        BuildingMode::Factory => {
            commands.spawn((
                GhostPreview,
                Mesh3d(meshes.add(Cuboid::new(1.5, 1.5, 1.5))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba(0.5, 0.5, 0.7, 0.5),
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                })),
                Transform::from_translation(Vec3::new(pos.x, 0.75, pos.z)),
            ));
        }
        BuildingMode::Shop => {
            commands.spawn((
                GhostPreview,
                Mesh3d(meshes.add(Cuboid::new(1.2, 1.2, 1.2))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba(0.8, 0.4, 0.6, 0.5),
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                })),
                Transform::from_translation(Vec3::new(pos.x, 0.6, pos.z)),
            ));
        }
        BuildingMode::None => {}
    }
}

/// System to handle placement clicks
pub fn handle_placement_click(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut building_state: ResMut<BuildingState>,
    mut sim_world: ResMut<SimWorldResource>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mappings: ResMut<EntityMappings>,
    // Check if mouse is over UI
    interaction_query: Query<&Interaction, With<Button>>,
) {
    // Don't place if clicking on UI
    for interaction in interaction_query.iter() {
        if *interaction == Interaction::Pressed || *interaction == Interaction::Hovered {
            return;
        }
    }

    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    if building_state.mode == BuildingMode::None {
        return;
    }

    let position = building_state
        .snapped_position
        .or(building_state.cursor_position);

    let Some(pos) = position else {
        return;
    };

    let world = &mut sim_world.0;

    match building_state.mode {
        BuildingMode::Road => {
            if let Some(start) = building_state.road_start {
                // Second click - create the road
                let snap_distance = building_state.snap_distance;
                
                // Try to add road with game cost checking
                let result = if world.game_state.is_some() {
                    world.try_add_road_at_positions(start, pos, snap_distance)
                } else {
                    world.add_road_at_positions(start, pos, snap_distance).map(Some)
                };
                
                match result {
                    Ok(Some((start_id, end_id, forward_road, _))) => {
                        // Spawn visuals for new intersection(s) if they don't exist
                        if !mappings.intersections.contains_key(&start_id) {
                            if let Some(intersection) = world.intersections.get(&start_id) {
                                spawn_intersection_visual(
                                    &mut commands,
                                    &mut meshes,
                                    &mut materials,
                                    start_id,
                                    &intersection.position,
                                    &mut mappings,
                                );
                            }
                        }
                        if !mappings.intersections.contains_key(&end_id) {
                            if let Some(intersection) = world.intersections.get(&end_id) {
                                spawn_intersection_visual(
                                    &mut commands,
                                    &mut meshes,
                                    &mut materials,
                                    end_id,
                                    &intersection.position,
                                    &mut mappings,
                                );
                            }
                        }

                        // Spawn road visual
                        if let Some(road) = world.road_network.get_road(forward_road) {
                            spawn_road_visual(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &world.road_network,
                                forward_road,
                                road,
                                &mut mappings,
                            );
                        }

                        bevy::log::info!("Created road between {:?} and {:?}", start_id, end_id);
                    }
                    Ok(None) => {
                        bevy::log::warn!("Insufficient funds to create road");
                    }
                    Err(e) => {
                        bevy::log::warn!("Failed to create road: {}", e);
                    }
                }
                building_state.road_start = None;
            } else {
                // First click - set start position
                building_state.road_start = Some(pos);
            }
        }
        BuildingMode::House | BuildingMode::Factory | BuildingMode::Shop => {
            // For buildings, find or create an intersection at this position
            let snap_distance = building_state.snap_distance;
            let intersection_id =
                match find_or_create_building_intersection(world, pos, snap_distance) {
                    Ok(id) => id,
                    Err(e) => {
                        bevy::log::warn!("Failed to create intersection for building: {}", e);
                        return;
                    }
                };

            // Spawn intersection visual if new
            if !mappings.intersections.contains_key(&intersection_id) {
                if let Some(intersection) = world.intersections.get(&intersection_id) {
                    spawn_intersection_visual(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        intersection_id,
                        &intersection.position,
                        &mut mappings,
                    );
                }
            }

            // Spawn the building using the helper
            spawn_building_at_intersection(
                building_state.mode,
                intersection_id,
                world,
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut mappings,
            );
        }
        BuildingMode::None => {}
    }
}

/// Helper to spawn a building at an intersection with its visual
fn spawn_building_at_intersection(
    building_mode: BuildingMode,
    intersection_id: crate::simulation::IntersectionId,
    world: &mut crate::simulation::SimWorld,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    mappings: &mut ResMut<EntityMappings>,
) {
    let position = match world.intersections.get(&intersection_id) {
        Some(intersection) => intersection.position,
        None => return,
    };

    match building_mode {
        BuildingMode::House => {
            let maybe_house_id = if world.game_state.is_some() {
                world.try_add_house(intersection_id)
            } else {
                Some(world.add_house(intersection_id))
            };
            
            if let Some(house_id) = maybe_house_id {
                spawn_house_visual(commands, meshes, materials, house_id, &position, mappings);
                bevy::log::info!("Created house at {:?}", intersection_id);
            } else {
                bevy::log::warn!("Insufficient funds to create house");
            }
        }
        BuildingMode::Factory => {
            let maybe_factory_id = if world.game_state.is_some() {
                world.try_add_factory(intersection_id)
            } else {
                Some(world.add_factory(intersection_id))
            };
            
            if let Some(factory_id) = maybe_factory_id {
                spawn_factory_visual(commands, meshes, materials, factory_id, &position, mappings);
                bevy::log::info!("Created factory at {:?}", intersection_id);
            } else {
                bevy::log::warn!("Insufficient funds to create factory");
            }
        }
        BuildingMode::Shop => {
            let maybe_shop_id = if world.game_state.is_some() {
                world.try_add_shop(intersection_id)
            } else {
                Some(world.add_shop(intersection_id))
            };
            
            if let Some(shop_id) = maybe_shop_id {
                spawn_shop_visual(commands, meshes, materials, shop_id, &position, mappings);
                bevy::log::info!("Created shop at {:?}", intersection_id);
            } else {
                bevy::log::warn!("Insufficient funds to create shop");
            }
        }
        _ => {}
    }
}

/// Helper to find or create an intersection for building placement
fn find_or_create_building_intersection(
    world: &mut crate::simulation::SimWorld,
    position: Position,
    snap_distance: f32,
) -> anyhow::Result<crate::simulation::IntersectionId> {
    // First check for existing intersection nearby
    if let Some(closest_id) = world.road_network.find_closest_intersection(&position) {
        if let Some(intersection_pos) = world.road_network.get_intersection_position(closest_id) {
            if position.distance(intersection_pos) <= snap_distance {
                return Ok(closest_id);
            }
        }
    }

    // Check if we're near a road that can be split
    if let Some((road_id, closest_point, _, _)) =
        world.road_network.find_closest_point_on_road(&position)
    {
        if position.distance(&closest_point) <= snap_distance {
            let (new_intersection, _, _) = world.split_road_at_position(road_id, closest_point)?;
            return Ok(new_intersection);
        }
    }

    // Create new intersection at position
    Ok(world.add_intersection(position))
}

/// Update button border colors to show current selection
pub fn update_button_borders(
    building_state: Res<BuildingState>,
    mut button_query: Query<(&BuildModeButton, &mut BorderColor)>,
) {
    if !building_state.is_changed() {
        return;
    }

    for (button, mut border_color) in button_query.iter_mut() {
        *border_color = BorderColor::all(if building_state.mode == button.0 {
            Color::srgb(0.0, 1.0, 0.0)
        } else {
            Color::WHITE
        });
    }
}
