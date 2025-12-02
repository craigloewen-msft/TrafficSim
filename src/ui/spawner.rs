//! Systems for spawning visual entities from simulation state

use bevy::prelude::*;

use super::components::{
    DemandIndicator, DeliveryIndicator, EntityMappings, FactoryLink, HouseLink, IntersectionLink,
    RoadLink, ShopLink, SimSynced, SimWorldResource,
};
use crate::simulation::SimRoadNetwork;
use crate::simulation::{FactoryId, HouseId, IntersectionId, Position, RoadId, ShopId, SimRoad};

/// System to create initial visual entities from simulation state
pub fn spawn_initial_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sim_world: Res<SimWorldResource>,
    mut mappings: ResMut<EntityMappings>,
) {
    let world = &sim_world.0;

    spawn_intersections(
        &mut commands,
        &mut meshes,
        &mut materials,
        world,
        &mut mappings,
    );
    spawn_roads(
        &mut commands,
        &mut meshes,
        &mut materials,
        world,
        &mut mappings,
    );
    spawn_houses(
        &mut commands,
        &mut meshes,
        &mut materials,
        world,
        &mut mappings,
    );
    spawn_factories(
        &mut commands,
        &mut meshes,
        &mut materials,
        world,
        &mut mappings,
    );
    spawn_shops(
        &mut commands,
        &mut meshes,
        &mut materials,
        world,
        &mut mappings,
    );
}

fn spawn_intersections(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    world: &crate::simulation::SimWorld,
    mappings: &mut ResMut<EntityMappings>,
) {
    for (id, intersection) in &world.intersections {
        spawn_intersection_visual(
            commands,
            meshes,
            materials,
            *id,
            &intersection.position,
            mappings,
        );
    }
}

/// Spawn a single intersection visual
pub fn spawn_intersection_visual(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    id: IntersectionId,
    pos: &Position,
    mappings: &mut ResMut<EntityMappings>,
) {
    const INTERSECTION_SIZE: f32 = 0.6;
    const INTERSECTION_HEIGHT: f32 = 0.03;
    let intersection_color = Color::srgb(0.3, 0.3, 0.3);

    let entity = commands
        .spawn((
            SimSynced,
            IntersectionLink(id),
            Mesh3d(meshes.add(Cuboid::new(
                INTERSECTION_SIZE,
                INTERSECTION_HEIGHT,
                INTERSECTION_SIZE,
            ))),
            MeshMaterial3d(materials.add(intersection_color)),
            Transform::from_translation(Vec3::new(pos.x, INTERSECTION_HEIGHT / 2.0, pos.z)),
        ))
        .id();
    mappings.intersections.insert(id, entity);
}

fn spawn_roads(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    world: &crate::simulation::SimWorld,
    mappings: &mut ResMut<EntityMappings>,
) {
    // Track which road pairs we've rendered (to avoid double-rendering two-way roads)
    let mut rendered_road_pairs: std::collections::HashSet<(
        crate::simulation::IntersectionId,
        crate::simulation::IntersectionId,
    )> = std::collections::HashSet::new();

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

        spawn_road_visual(
            commands,
            meshes,
            materials,
            &world.road_network,
            *id,
            road,
            mappings,
        );
    }
}

/// Spawn a single road visual
pub fn spawn_road_visual(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &SimRoadNetwork,
    id: RoadId,
    road: &SimRoad,
    mappings: &mut ResMut<EntityMappings>,
) {
    const TWO_WAY_ROAD_WIDTH: f32 = 0.6;
    const ROAD_HEIGHT: f32 = 0.02;
    let road_color = Color::srgb(0.2, 0.2, 0.2);

    let start_pos = road_network.get_intersection_position(road.start_intersection);
    let end_pos = road_network.get_intersection_position(road.end_intersection);

    if let (Some(start), Some(end)) = (start_pos, end_pos) {
        let length = start.distance(end);
        let midpoint = Position::new(
            (start.x + end.x) / 2.0,
            (start.y + end.y) / 2.0,
            (start.z + end.z) / 2.0,
        );
        let angle = start.angle_to(end);
        let rotation = Quat::from_rotation_y(angle);
        let width = if road.is_two_way {
            TWO_WAY_ROAD_WIDTH
        } else {
            0.4
        };

        let entity = commands
            .spawn((
                SimSynced,
                RoadLink(id),
                Mesh3d(meshes.add(Cuboid::new(width, ROAD_HEIGHT, length))),
                MeshMaterial3d(materials.add(road_color)),
                Transform::from_translation(Vec3::new(midpoint.x, ROAD_HEIGHT / 2.0, midpoint.z))
                    .with_rotation(rotation),
            ))
            .id();
        mappings.roads.insert(id, entity);

        // Add direction arrows
        spawn_direction_arrows(
            commands,
            meshes,
            materials,
            start,
            end,
            if road.is_two_way { -0.15 } else { 0.0 },
            entity,
            false,
        );

        if road.is_two_way {
            spawn_direction_arrows(commands, meshes, materials, end, start, 0.15, entity, true);
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
                Mesh3d(meshes.add(Cuboid::new(
                    ARROW_ARM_WIDTH,
                    ARROW_ARM_HEIGHT,
                    ARROW_ARM_LENGTH,
                ))),
                MeshMaterial3d(materials.add(arrow_color)),
                Transform::from_translation(Vec3::new(
                    offset_x - ARROW_ARM_LENGTH * 0.5 * ARROW_ANGLE.sin(),
                    ARROW_ARM_HEIGHT,
                    z_offset + ARROW_ARM_LENGTH * 0.5 * ARROW_ANGLE.cos(),
                ))
                .with_rotation(Quat::from_rotation_y(-ARROW_ANGLE + arrow_angle_offset)),
            ));

            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(
                    ARROW_ARM_WIDTH,
                    ARROW_ARM_HEIGHT,
                    ARROW_ARM_LENGTH,
                ))),
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

fn spawn_houses(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    world: &crate::simulation::SimWorld,
    mappings: &mut ResMut<EntityMappings>,
) {
    for (id, house) in &world.houses {
        if let Some(intersection) = world.intersections.get(&house.intersection_id) {
            spawn_house_visual(
                commands,
                meshes,
                materials,
                *id,
                &intersection.position,
                mappings,
            );
        }
    }
}

/// Spawn a single house visual
pub fn spawn_house_visual(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    id: HouseId,
    pos: &Position,
    mappings: &mut ResMut<EntityMappings>,
) {
    const HOUSE_SIZE: f32 = 1.0;
    let house_color = Color::srgb(0.7, 0.6, 0.4);

    let entity = commands
        .spawn((
            SimSynced,
            HouseLink(id),
            Mesh3d(meshes.add(Cuboid::new(HOUSE_SIZE, HOUSE_SIZE, HOUSE_SIZE))),
            MeshMaterial3d(materials.add(house_color)),
            Transform::from_translation(Vec3::new(pos.x, HOUSE_SIZE / 2.0, pos.z)),
        ))
        .id();
    mappings.houses.insert(id, entity);

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

fn spawn_factories(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    world: &crate::simulation::SimWorld,
    mappings: &mut ResMut<EntityMappings>,
) {
    for (id, factory) in &world.factories {
        if let Some(intersection) = world.intersections.get(&factory.intersection_id) {
            spawn_factory_visual(
                commands,
                meshes,
                materials,
                *id,
                &intersection.position,
                mappings,
            );
        }
    }
}

/// Spawn a single factory visual
pub fn spawn_factory_visual(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    id: FactoryId,
    pos: &Position,
    mappings: &mut ResMut<EntityMappings>,
) {
    const FACTORY_SIZE: f32 = 1.5;
    let factory_color = Color::srgb(0.5, 0.5, 0.7);

    let entity = commands
        .spawn((
            SimSynced,
            FactoryLink(id),
            Mesh3d(meshes.add(Cuboid::new(FACTORY_SIZE, FACTORY_SIZE, FACTORY_SIZE))),
            MeshMaterial3d(materials.add(factory_color)),
            Transform::from_translation(Vec3::new(pos.x, FACTORY_SIZE / 2.0, pos.z)),
        ))
        .id();
    mappings.factories.insert(id, entity);

    // Add demand indicator (top sphere)
    let indicator = commands
        .spawn((
            DemandIndicator,
            Mesh3d(meshes.add(Sphere::new(0.25))),
            MeshMaterial3d(materials.add(Color::srgb(0.0, 1.0, 0.0))),
            Transform::from_translation(Vec3::new(0.0, 1.5, 0.0)),
        ))
        .id();
    commands.entity(entity).add_child(indicator);

    // Add delivery count indicators (side spheres - max 2)
    for i in 0..2 {
        let delivery_indicator = commands
            .spawn((
                DeliveryIndicator,
                Mesh3d(meshes.add(Sphere::new(0.15))),
                MeshMaterial3d(materials.add(Color::srgb(0.3, 0.3, 0.3))), // Dark gray by default
                Transform::from_translation(Vec3::new(0.9, 0.3 + i as f32 * 0.4, 0.0)),
            ))
            .id();
        commands.entity(entity).add_child(delivery_indicator);
    }
}

fn spawn_shops(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    world: &crate::simulation::SimWorld,
    mappings: &mut ResMut<EntityMappings>,
) {
    for (id, shop) in &world.shops {
        if let Some(intersection) = world.intersections.get(&shop.intersection_id) {
            spawn_shop_visual(
                commands,
                meshes,
                materials,
                *id,
                &intersection.position,
                mappings,
            );
        }
    }
}

/// Spawn a single shop visual
pub fn spawn_shop_visual(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    id: ShopId,
    pos: &Position,
    mappings: &mut ResMut<EntityMappings>,
) {
    const SHOP_SIZE: f32 = 1.2;
    let shop_color = Color::srgb(0.8, 0.4, 0.6);

    let entity = commands
        .spawn((
            SimSynced,
            ShopLink(id),
            Mesh3d(meshes.add(Cuboid::new(SHOP_SIZE, SHOP_SIZE, SHOP_SIZE))),
            MeshMaterial3d(materials.add(shop_color)),
            Transform::from_translation(Vec3::new(pos.x, SHOP_SIZE / 2.0, pos.z)),
        ))
        .id();
    mappings.shops.insert(id, entity);

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
