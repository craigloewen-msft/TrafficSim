use bevy::prelude::*;
use anyhow::Result;

use crate::intersection::{IntersectionEntity, spawn_intersection};
use crate::road::RoadEntity;
use crate::road_network::RoadNetwork;

#[derive(Component, Debug)]
pub struct House {}

pub fn spawn_house_intersection(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &mut ResMut<RoadNetwork>,
    position: Vec3,
) -> Result<IntersectionEntity> {
    const HOUSE_SIZE: f32 = 1.0;
    let house_color = Color::srgb(0.7, 0.6, 0.4);

    let intersection_entity = spawn_intersection(
        commands,
        meshes,
        materials,
        road_network,
        position,
    )?;

    commands.entity(intersection_entity.0).insert((
        House {},
        Mesh3d(meshes.add(Cuboid::new(HOUSE_SIZE, HOUSE_SIZE, HOUSE_SIZE))),
        MeshMaterial3d(materials.add(house_color)),
    ));

    Ok(intersection_entity)
}

pub fn spawn_house_with_driveway(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &mut ResMut<RoadNetwork>,
    house_position: Vec3,
    road_intersection_entity: IntersectionEntity,
    road_intersection_position: Vec3,
) -> Result<IntersectionEntity> {
    let house_intersection_entity = spawn_house_intersection(
        commands,
        meshes,
        materials,
        road_network,
        house_position,
    )?;

    spawn_driveway(
        commands,
        meshes,
        materials,
        road_network,
        house_intersection_entity,
        road_intersection_entity,
        house_position,
        road_intersection_position,
    )?;

    Ok(house_intersection_entity)
}

fn spawn_driveway(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    road_network: &mut ResMut<RoadNetwork>,
    house_intersection: IntersectionEntity,
    road_intersection: IntersectionEntity,
    house_pos: Vec3,
    road_pos: Vec3,
) -> Result<RoadEntity> {
    const DRIVEWAY_WIDTH: f32 = 0.3;
    const DRIVEWAY_HEIGHT: f32 = 0.02;
    let driveway_color = Color::srgb(0.25, 0.25, 0.25);

    let direction = (road_pos - house_pos).normalize();
    let angle = direction.x.atan2(direction.z);
    let length = house_pos.distance(road_pos);
    let midpoint = (house_pos + road_pos) / 2.0;
    let rotation = Quat::from_rotation_y(angle);

    let driveway_entity = commands.spawn((
        crate::road::Road {
            start_intersection_entity: house_intersection,
            end_intersection_entity: road_intersection,
            angle,
        },
        Mesh3d(meshes.add(Cuboid::new(DRIVEWAY_WIDTH, DRIVEWAY_HEIGHT, length))),
        MeshMaterial3d(materials.add(driveway_color)),
        Transform::from_translation(Vec3::new(midpoint.x, DRIVEWAY_HEIGHT / 2.0, midpoint.z))
            .with_rotation(rotation),
    )).id();

    let driveway_entity_wrapper = RoadEntity(driveway_entity);

    road_network.add_road(
        driveway_entity_wrapper,
        house_intersection,
        road_intersection,
    );

    Ok(driveway_entity_wrapper)
}

pub fn update_houses(_time: Res<Time>, _house_query: Query<(&House, &Transform)>) {
}

pub struct HousePlugin;

impl Plugin for HousePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_houses);
    }
}
