//! UI module that visualizes the simulation state using Bevy
//! 
//! This module is purely for visualization - all simulation logic is in the `simulation` module.
//! The UI reads state from `SimWorld` and renders it using Bevy's 3D graphics.

use bevy::prelude::*;
use std::collections::HashMap;

use crate::simulation::{
    CarId, FactoryId, HouseId, IntersectionId, Position, RoadId, ShopId, SimWorld,
};

/// Resource wrapper for the simulation world
#[derive(Resource)]
pub struct SimWorldResource(pub SimWorld);

impl Default for SimWorldResource {
    fn default() -> Self {
        Self(SimWorld::create_test_world())
    }
}

/// Marker component for ground plane
#[derive(Component)]
pub struct Ground;

/// Marker for entities synced from simulation
#[derive(Component)]
pub struct SimSynced;

/// Links a Bevy entity to a simulation intersection
#[derive(Component)]
pub struct IntersectionLink(pub IntersectionId);

/// Links a Bevy entity to a simulation road
#[derive(Component)]
pub struct RoadLink(pub RoadId);

/// Links a Bevy entity to a simulation car
#[derive(Component)]
pub struct CarLink(pub CarId);

/// Links a Bevy entity to a simulation house
#[derive(Component)]
pub struct HouseLink(pub HouseId);

/// Links a Bevy entity to a simulation factory
#[derive(Component)]
pub struct FactoryLink(pub FactoryId);

/// Links a Bevy entity to a simulation shop
#[derive(Component)]
pub struct ShopLink(pub ShopId);

/// Component to mark the visual demand indicator entity
#[derive(Component)]
pub struct DemandIndicator;

/// Resource to track Bevy entities mapped to simulation entities
#[derive(Resource, Default)]
pub struct EntityMappings {
    pub intersections: HashMap<IntersectionId, Entity>,
    pub roads: HashMap<RoadId, Entity>,
    pub cars: HashMap<CarId, Entity>,
    pub houses: HashMap<HouseId, Entity>,
    pub factories: HashMap<FactoryId, Entity>,
    pub shops: HashMap<ShopId, Entity>,
}

/// System to setup the world environment (ground, lighting, camera)
pub fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn a 3D camera with top-down view
    commands.spawn((
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

/// System to create initial visual entities from simulation state
pub fn spawn_initial_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sim_world: Res<SimWorldResource>,
    mut mappings: ResMut<EntityMappings>,
) {
    let world = &sim_world.0;

    // Spawn intersections
    const INTERSECTION_SIZE: f32 = 0.6;
    const INTERSECTION_HEIGHT: f32 = 0.03;
    let intersection_color = Color::srgb(0.3, 0.3, 0.3);

    for (id, intersection) in &world.intersections {
        let pos = &intersection.position;
        let entity = commands
            .spawn((
                SimSynced,
                IntersectionLink(*id),
                Mesh3d(meshes.add(Cuboid::new(INTERSECTION_SIZE, INTERSECTION_HEIGHT, INTERSECTION_SIZE))),
                MeshMaterial3d(materials.add(intersection_color)),
                Transform::from_translation(Vec3::new(pos.x, INTERSECTION_HEIGHT / 2.0, pos.z)),
            ))
            .id();
        mappings.intersections.insert(*id, entity);
    }

    // Spawn roads
    const TWO_WAY_ROAD_WIDTH: f32 = 0.6;
    const ROAD_HEIGHT: f32 = 0.02;
    let road_color = Color::srgb(0.2, 0.2, 0.2);

    // Track which road pairs we've rendered (to avoid double-rendering two-way roads)
    let mut rendered_road_pairs: std::collections::HashSet<(IntersectionId, IntersectionId)> = std::collections::HashSet::new();

    for (id, road) in world.road_network.get_all_roads() {
        // For two-way roads, only render once per pair
        let pair_key = if road.start_intersection.0 .0 < road.end_intersection.0 .0 {
            (road.start_intersection, road.end_intersection)
        } else {
            (road.end_intersection, road.start_intersection)
        };

        if road.is_two_way && rendered_road_pairs.contains(&pair_key) {
            continue;
        }
        if road.is_two_way {
            rendered_road_pairs.insert(pair_key);
        }

        let start_pos = world.road_network.get_intersection_position(road.start_intersection);
        let end_pos = world.road_network.get_intersection_position(road.end_intersection);

        if let (Some(start), Some(end)) = (start_pos, end_pos) {
            let length = start.distance(end);
            let midpoint = Position::new(
                (start.x + end.x) / 2.0,
                (start.y + end.y) / 2.0,
                (start.z + end.z) / 2.0,
            );
            let angle = start.angle_to(end);
            let rotation = Quat::from_rotation_y(angle);
            let width = if road.is_two_way { TWO_WAY_ROAD_WIDTH } else { 0.4 };

            let entity = commands
                .spawn((
                    SimSynced,
                    RoadLink(*id),
                    Mesh3d(meshes.add(Cuboid::new(width, ROAD_HEIGHT, length))),
                    MeshMaterial3d(materials.add(road_color)),
                    Transform::from_translation(Vec3::new(midpoint.x, ROAD_HEIGHT / 2.0, midpoint.z))
                        .with_rotation(rotation),
                ))
                .id();
            mappings.roads.insert(*id, entity);

            // Add direction arrows
            spawn_direction_arrows(
                &mut commands,
                &mut meshes,
                &mut materials,
                start,
                end,
                if road.is_two_way { -0.15 } else { 0.0 },
                entity,
                false,
            );
            
            if road.is_two_way {
                spawn_direction_arrows(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    end,
                    start,
                    0.15,
                    entity,
                    true,
                );
            }
        }
    }

    // Spawn houses
    const HOUSE_SIZE: f32 = 1.0;
    let house_color = Color::srgb(0.7, 0.6, 0.4);

    for (id, house) in &world.houses {
        if let Some(intersection) = world.intersections.get(&house.intersection_id) {
            let pos = &intersection.position;
            let entity = commands
                .spawn((
                    SimSynced,
                    HouseLink(*id),
                    Mesh3d(meshes.add(Cuboid::new(HOUSE_SIZE, HOUSE_SIZE, HOUSE_SIZE))),
                    MeshMaterial3d(materials.add(house_color)),
                    Transform::from_translation(Vec3::new(pos.x, HOUSE_SIZE / 2.0, pos.z)),
                ))
                .id();
            mappings.houses.insert(*id, entity);

            // Add demand indicator
            let indicator = commands
                .spawn((
                    DemandIndicator,
                    Mesh3d(meshes.add(Sphere::new(0.2))),
                    MeshMaterial3d(materials.add(Color::srgb(0.0, 1.0, 0.0))),
                    Transform::from_translation(Vec3::new(0.0, 1.2, 0.0)),
                ))
                .id();
            commands.entity(entity).add_child(indicator);
        }
    }

    // Spawn factories
    const FACTORY_SIZE: f32 = 1.5;
    let factory_color = Color::srgb(0.5, 0.5, 0.7);

    for (id, factory) in &world.factories {
        if let Some(intersection) = world.intersections.get(&factory.intersection_id) {
            let pos = &intersection.position;
            let entity = commands
                .spawn((
                    SimSynced,
                    FactoryLink(*id),
                    Mesh3d(meshes.add(Cuboid::new(FACTORY_SIZE, FACTORY_SIZE, FACTORY_SIZE))),
                    MeshMaterial3d(materials.add(factory_color)),
                    Transform::from_translation(Vec3::new(pos.x, FACTORY_SIZE / 2.0, pos.z)),
                ))
                .id();
            mappings.factories.insert(*id, entity);

            // Add demand indicator
            let indicator = commands
                .spawn((
                    DemandIndicator,
                    Mesh3d(meshes.add(Sphere::new(0.25))),
                    MeshMaterial3d(materials.add(Color::srgb(0.0, 1.0, 0.0))),
                    Transform::from_translation(Vec3::new(0.0, 1.5, 0.0)),
                ))
                .id();
            commands.entity(entity).add_child(indicator);
        }
    }

    // Spawn shops
    const SHOP_SIZE: f32 = 1.2;
    let shop_color = Color::srgb(0.8, 0.4, 0.6);

    for (id, shop) in &world.shops {
        if let Some(intersection) = world.intersections.get(&shop.intersection_id) {
            let pos = &intersection.position;
            let entity = commands
                .spawn((
                    SimSynced,
                    ShopLink(*id),
                    Mesh3d(meshes.add(Cuboid::new(SHOP_SIZE, SHOP_SIZE, SHOP_SIZE))),
                    MeshMaterial3d(materials.add(shop_color)),
                    Transform::from_translation(Vec3::new(pos.x, SHOP_SIZE / 2.0, pos.z)),
                ))
                .id();
            mappings.shops.insert(*id, entity);

            // Add demand indicator
            let indicator = commands
                .spawn((
                    DemandIndicator,
                    Mesh3d(meshes.add(Sphere::new(0.22))),
                    MeshMaterial3d(materials.add(Color::srgb(0.0, 1.0, 0.0))),
                    Transform::from_translation(Vec3::new(0.0, 1.3, 0.0)),
                ))
                .id();
            commands.entity(entity).add_child(indicator);
        }
    }
}

/// Helper function to spawn V-shaped directional arrow indicators on a road
fn spawn_direction_arrows(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    start_pos: &Position,
    end_pos: &Position,
    offset_x: f32,
    parent_entity: Entity,
    reverse_direction: bool,
) {
    const ARROW_ARM_WIDTH: f32 = 0.04;
    const ARROW_ARM_HEIGHT: f32 = 0.03;
    const ARROW_ARM_LENGTH: f32 = 0.15;
    const ARROW_ANGLE: f32 = 0.5;
    const ARROW_SPACING: f32 = 2.0;
    let arrow_color = Color::srgb(0.9, 0.9, 0.3);

    let length = start_pos.distance(end_pos);
    let arrow_angle_offset = if reverse_direction {
        0.0
    } else {
        std::f32::consts::PI / 2.0
    };

    let num_arrows = (length / ARROW_SPACING).max(1.0) as i32;

    for i in 0..num_arrows {
        let t = (i as f32 + 0.5) / num_arrows as f32;
        let z_offset = (t - 0.5) * length;

        commands.entity(parent_entity).with_children(|parent| {
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(ARROW_ARM_WIDTH, ARROW_ARM_HEIGHT, ARROW_ARM_LENGTH))),
                MeshMaterial3d(materials.add(arrow_color)),
                Transform::from_translation(Vec3::new(
                    offset_x - ARROW_ARM_LENGTH * 0.5 * ARROW_ANGLE.sin(),
                    ARROW_ARM_HEIGHT,
                    z_offset + ARROW_ARM_LENGTH * 0.5 * ARROW_ANGLE.cos(),
                ))
                .with_rotation(Quat::from_rotation_y(-ARROW_ANGLE + arrow_angle_offset)),
            ));

            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(ARROW_ARM_WIDTH, ARROW_ARM_HEIGHT, ARROW_ARM_LENGTH))),
                MeshMaterial3d(materials.add(arrow_color)),
                Transform::from_translation(Vec3::new(
                    offset_x + ARROW_ARM_LENGTH * 0.5 * ARROW_ANGLE.sin(),
                    ARROW_ARM_HEIGHT,
                    z_offset + ARROW_ARM_LENGTH * 0.5 * ARROW_ANGLE.cos(),
                ))
                .with_rotation(Quat::from_rotation_y(ARROW_ANGLE + arrow_angle_offset)),
            ));
        });
    }
}

/// System to run simulation tick
pub fn tick_simulation(
    time: Res<Time>,
    mut sim_world: ResMut<SimWorldResource>,
) {
    sim_world.0.tick(time.delta_secs());
}

/// System to sync car visuals from simulation state
pub fn sync_cars(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sim_world: Res<SimWorldResource>,
    mut mappings: ResMut<EntityMappings>,
    mut car_query: Query<(Entity, &CarLink, &mut Transform)>,
) {
    let world = &sim_world.0;
    const CAR_LENGTH: f32 = 0.5;

    // Update existing cars and track which ones still exist
    let mut existing_car_ids: std::collections::HashSet<CarId> = std::collections::HashSet::new();
    
    for (entity, link, mut transform) in car_query.iter_mut() {
        if let Some(car) = world.cars.get(&link.0) {
            existing_car_ids.insert(link.0);
            transform.translation = Vec3::new(car.position.x, 0.3, car.position.z);
            transform.rotation = Quat::from_rotation_y(car.angle);
        } else {
            // Car no longer exists in simulation, despawn
            commands.entity(entity).despawn_recursive();
            mappings.cars.remove(&link.0);
        }
    }

    // Spawn new cars
    for (id, car) in &world.cars {
        if !existing_car_ids.contains(id) {
            let entity = commands
                .spawn((
                    SimSynced,
                    CarLink(*id),
                    Mesh3d(meshes.add(Cuboid::new(0.3, 0.2, CAR_LENGTH))),
                    MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))),
                    Transform::from_translation(Vec3::new(car.position.x, 0.3, car.position.z))
                        .with_rotation(Quat::from_rotation_y(car.angle)),
                ))
                .id();
            mappings.cars.insert(*id, entity);
        }
    }
}

/// System to update factory demand indicators
pub fn update_factory_indicators(
    sim_world: Res<SimWorldResource>,
    factory_query: Query<(&FactoryLink, &Children)>,
    mut indicator_query: Query<&mut MeshMaterial3d<StandardMaterial>, With<DemandIndicator>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const LABOR_DEMAND_THRESHOLD: f32 = 10.0;
    
    for (link, children) in factory_query.iter() {
        if let Some(factory) = sim_world.0.factories.get(&link.0) {
            for child in children.iter() {
                if let Ok(material_handle) = indicator_query.get_mut(child) {
                    if let Some(material) = materials.get_mut(&material_handle.0) {
                        let demand_ratio = (factory.labor_demand / (LABOR_DEMAND_THRESHOLD * 2.0)).min(1.0);
                        if demand_ratio < 0.5 {
                            let t = demand_ratio * 2.0;
                            material.base_color = Color::srgb(t, 1.0, 0.0);
                        } else {
                            let t = (demand_ratio - 0.5) * 2.0;
                            material.base_color = Color::srgb(1.0, 1.0 - t, 0.0);
                        }
                    }
                }
            }
        }
    }
}

/// System to update shop demand indicators
pub fn update_shop_indicators(
    sim_world: Res<SimWorldResource>,
    shop_query: Query<(&ShopLink, &Children)>,
    mut indicator_query: Query<&mut MeshMaterial3d<StandardMaterial>, With<DemandIndicator>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const PRODUCT_DEMAND_THRESHOLD: f32 = 10.0;
    
    for (link, children) in shop_query.iter() {
        if let Some(shop) = sim_world.0.shops.get(&link.0) {
            for child in children.iter() {
                if let Ok(material_handle) = indicator_query.get_mut(child) {
                    if let Some(material) = materials.get_mut(&material_handle.0) {
                        let demand_ratio = (shop.product_demand / (PRODUCT_DEMAND_THRESHOLD * 2.0)).min(1.0);
                        if demand_ratio < 0.5 {
                            let t = demand_ratio * 2.0;
                            material.base_color = Color::srgb(t, 1.0, 0.0);
                        } else {
                            let t = (demand_ratio - 0.5) * 2.0;
                            material.base_color = Color::srgb(1.0, 1.0 - t, 0.0);
                        }
                    }
                }
            }
        }
    }
}

/// Handle basic keyboard input
pub fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

/// Plugin to register all UI systems
pub struct TrafficSimUIPlugin;

impl Plugin for TrafficSimUIPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimWorldResource>()
            .init_resource::<EntityMappings>()
            .add_systems(Startup, (setup_world, spawn_initial_visuals.after(setup_world)))
            .add_systems(FixedUpdate, tick_simulation)
            .add_systems(
                Update,
                (
                    sync_cars,
                    update_factory_indicators,
                    update_shop_indicators,
                    handle_input,
                ),
            );
    }
}
