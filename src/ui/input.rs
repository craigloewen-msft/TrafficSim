//! Input handling systems

use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;

use super::components::{CameraSettings, MainCamera};

/// Handle basic keyboard input
pub fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

/// Handle camera orbital rotation with mouse drag
/// 
/// Controls:
/// - Click and drag: Orbit camera around the point where camera looks at the ground
pub fn handle_camera_mouse(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
) {
    let Ok(mut transform) = camera_query.single_mut() else {
        return;
    };

    // Only rotate when left mouse button is pressed
    if !mouse_button.pressed(MouseButton::Left) {
        return;
    }

    // Accumulate all mouse motion events this frame
    let mut delta = Vec2::ZERO;
    for event in mouse_motion.read() {
        delta += event.delta;
    }

    if delta == Vec2::ZERO {
        return;
    }

    // Scale mouse movement for rotation sensitivity
    let sensitivity = 0.003;
    let yaw = -delta.x * sensitivity;
    let pitch = -delta.y * sensitivity;

    // Cast a ray from camera to find intersection with ground plane
    let ray_origin = transform.translation;
    let ray_direction = transform.forward().as_vec3();
    
    // Calculate intersection with ground plane (y = 0)
    // ray_origin.y + t * ray_direction.y = 0
    // t = -ray_origin.y / ray_direction.y
    let pivot_point = if ray_direction.y.abs() > 0.001 {
        let t = -ray_origin.y / ray_direction.y;
        if t > 0.0 {
            ray_origin + ray_direction * t
        } else {
            // If ray doesn't hit ground in front, use origin
            Vec3::ZERO
        }
    } else {
        // Ray parallel to ground, use origin
        Vec3::ZERO
    };

    // Rotate around Y axis (horizontal mouse movement)
    if yaw.abs() > 0.0001 {
        // Translate to pivot, rotate, translate back
        let to_pivot = transform.translation - pivot_point;
        let rotation = Quat::from_rotation_y(yaw);
        let rotated_offset = rotation * to_pivot;
        transform.translation = pivot_point + rotated_offset;
        transform.rotation = rotation * transform.rotation;
    }

    // Rotate around X axis (vertical mouse movement)
    if pitch.abs() > 0.0001 {
        // Translate to pivot, rotate, translate back
        let to_pivot = transform.translation - pivot_point;
        let rotation = Quat::from_rotation_x(pitch);
        let rotated_offset = rotation * to_pivot;
        
        // Check if new position is valid (not too low)
        let new_position = pivot_point + rotated_offset;
        if new_position.y > 5.0 {
            transform.translation = new_position;
            transform.rotation = rotation * transform.rotation;
        }
    }
}

/// Handle camera movement with keyboard input
/// 
/// Controls:
/// - WASD: Move camera horizontally
/// - Q/E: Rotate camera around the center
/// - Z/X: Zoom in/out (move camera up/down)
pub fn handle_camera_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    settings: Res<CameraSettings>,
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
) {
    let Ok(mut transform) = camera_query.single_mut() else {
        return;
    };

    let delta = time.delta_secs();
    
    // Calculate movement direction (in the camera's local space)
    let mut movement = Vec3::ZERO;
    
    // Forward/backward (W/S) - move along Z axis
    if keyboard.pressed(KeyCode::KeyW) {
        movement.z += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        movement.z -= 1.0;
    }
    
    // Left/right (A/D) - move along X axis
    if keyboard.pressed(KeyCode::KeyA) {
        movement.x += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        movement.x -= 1.0;
    }
    
    // Apply movement
    if movement != Vec3::ZERO {
        movement = movement.normalize() * settings.movement_speed * delta;
        transform.translation += movement;
    }
    
    // Zoom in/out (Z/X) - move camera up/down
    if keyboard.pressed(KeyCode::KeyZ) {
        transform.translation.y -= settings.zoom_speed * delta;
        // Clamp minimum height
        transform.translation.y = transform.translation.y.max(10.0);
    }
    if keyboard.pressed(KeyCode::KeyX) {
        transform.translation.y += settings.zoom_speed * delta;
        // Clamp maximum height
        transform.translation.y = transform.translation.y.min(200.0);
    }
    
    // Rotation around center (Q/E)
    if keyboard.pressed(KeyCode::KeyQ) {
        // Rotate counterclockwise around Y axis
        let rotation = Quat::from_rotation_y(settings.rotation_speed * delta);
        transform.translation = rotation * transform.translation;
        transform.rotation = rotation * transform.rotation;
    }
    if keyboard.pressed(KeyCode::KeyE) {
        // Rotate clockwise around Y axis
        let rotation = Quat::from_rotation_y(-settings.rotation_speed * delta);
        transform.translation = rotation * transform.translation;
        transform.rotation = rotation * transform.rotation;
    }
}
