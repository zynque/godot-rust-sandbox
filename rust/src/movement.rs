use godot::prelude::*;
use crate::physics::{GodotPhysics, GodotPhysicsSpace};

/// Tuning knobs for `move_and_slide`.
pub struct MoveAndSlideParams {
    pub max_slides: i32,
    /// cos(floor_max_angle). Default: cos(45°) ≈ 0.707
    pub floor_max_angle_cos: f32,
    /// Distance to snap downward when the body was on the floor last
    /// frame but isn't anymore (keeps it grounded over small bumps).
    pub floor_snap_length: f32,
    pub up_direction: Vector2,
}

impl Default for MoveAndSlideParams {
    fn default() -> Self {
        Self {
            max_slides: 6,
            floor_max_angle_cos: 0.70710678, // cos(45°)
            floor_snap_length: 4.0,
            up_direction: Vector2::UP,
        }
    }
}

/// Performs a Godot-style move-and-slide loop (modelled on CharacterBody2D).
///
/// * `physics`       – the custom physics space wrapper
/// * `body_rid`      – RID of the kinematic body to move
/// * `position`      – current world position (will be updated in-place)
/// * `velocity`      – current velocity (will be slid along surfaces)
/// * `was_on_floor`  – whether the body was on a floor surface last frame
/// * `delta`         – frame delta time
/// * `params`        – tuning constants
///
/// Returns `true` if the body is on a floor surface after the move.
pub fn move_and_slide(
    physics: &mut GodotPhysics,
    body_rid: Rid,
    position: &mut Vector2,
    velocity: &mut Vector2,
    was_on_floor: bool,
    delta: f32,
    params: &MoveAndSlideParams,
) -> bool {
    let mut motion = *velocity * delta;
    let mut on_floor = false;

    for _slide in 0..params.max_slides {
        if motion.length_squared() < 1e-6 {
            break;
        }

        let result = physics.body_test_motion(body_rid, *position, motion);

        if !result.collided {
            // travel includes any overlap-recovery displacement,
            // so always prefer it over raw motion.
            *position += result.travel;
            break;
        }

        // Apply the safe portion of the motion
        *position += result.travel;

        let normal = result.collision_normal;

        // Classify surface by comparing normal with up direction
        if normal.dot(params.up_direction) > params.floor_max_angle_cos {
            on_floor = true;
        }

        // Slide the remaining motion along the collision surface
        //   slide(v, n) = v − (v · n) × n
        motion = result.remainder - normal * result.remainder.dot(normal);

        // Slide velocity so it doesn't accumulate into the surface
        let vel_into_surface = velocity.dot(normal);
        if vel_into_surface < 0.0 {
            *velocity -= normal * vel_into_surface;
        }
    }

    // Clamp near-zero velocity components to avoid drift
    if velocity.x.abs() < 0.001 {
        velocity.x = 0.0;
    }
    if velocity.y.abs() < 0.001 {
        velocity.y = 0.0;
    }

    // --- Floor snap (keeps body grounded over small bumps / edges) ---
    if was_on_floor && !on_floor && velocity.y >= 0.0 {
        let snap_result = physics.body_test_motion(
            body_rid,
            *position,
            Vector2::new(0.0, params.floor_snap_length),
        );
        if snap_result.collided {
            let snap_normal = snap_result.collision_normal;
            if snap_normal.dot(params.up_direction) > params.floor_max_angle_cos {
                *position += snap_result.travel;
                on_floor = true;
            }
        }
    }

    on_floor
}
