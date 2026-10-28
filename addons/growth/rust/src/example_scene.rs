//! The demo scene that used to be hardcoded in the parcel renderer.

use godot::builtin::Vector3;

use crate::parcels::{Parcel, ParcelShape};
use crate::tree::generate_tree_parcels;

/// Thin parcels marking the world axes, for orientation while debugging.
pub fn coordinate_axis_parcels() -> Vec<Parcel> {
    let axis_x = ParcelShape {
        forward: Vector3::RIGHT,
        normal: Vector3::UP,
        axis_length: 18.0,
        width: 0.01,
        height: 0.01,
    };
    let axis_y = ParcelShape {
        forward: Vector3::UP,
        normal: Vector3::BACK,
        axis_length: 18.0,
        width: 0.01,
        height: 0.01,
    };
    let axis_z = ParcelShape {
        forward: Vector3::BACK,
        normal: Vector3::UP,
        axis_length: 18.0,
        width: 0.01,
        height: 0.01,
    };

    vec![
        Parcel::new(
            Vector3::new(-4.0, 0.0, -5.0),
            axis_x,
            1.0,
            Vector3::new(1.0, 0.0, 0.0),
        ),
        Parcel::new(
            Vector3::new(0.0, -4.0, -5.0),
            axis_y,
            1.0,
            Vector3::new(0.0, 1.0, 0.0),
        ),
        Parcel::new(
            Vector3::new(0.0, 0.0, -7.0),
            axis_z,
            1.0,
            Vector3::new(0.0, 0.2, 1.0),
        ),
    ]
}

/// Three overlapping green parcels that show how densities blend. The two
/// blobs slide apart and back together over time.
pub fn example_parcels(time_seconds: f32) -> Vec<Parcel> {
    let motion = (0.8 * time_seconds).cos();
    let blob = ParcelShape {
        forward: Vector3::BACK,
        normal: Vector3::UP,
        axis_length: 0.3,
        width: 0.3,
        height: 0.02,
    };
    let slab = ParcelShape {
        forward: Vector3::BACK,
        normal: Vector3::UP,
        axis_length: 0.3,
        width: 0.02,
        height: 0.05,
    };
    let color = Vector3::new(0.25, 0.75, 0.35);

    vec![
        Parcel::new(
            Vector3::new(4.0 - 0.6 * motion, 2.0, -4.0),
            blob,
            0.8,
            color,
        ),
        Parcel::new(
            Vector3::new(4.0 + 0.6 * motion, 2.0, -4.0),
            blob,
            0.8,
            color,
        ),
        Parcel::new(Vector3::new(4.0, 2.0, -4.0), slab, 0.8, color),
    ]
}

/// Everything the renderer demo needs: coordinate axes, the example parcels,
/// and the tree, at the given scene time.
pub fn example_scene_parcels_at(time_seconds: f32) -> Vec<Parcel> {
    let mut parcels = coordinate_axis_parcels();
    parcels.extend(example_parcels(time_seconds));
    parcels.extend(generate_tree_parcels());
    parcels
}

/// The demo scene at time zero.
pub fn example_scene_parcels() -> Vec<Parcel> {
    example_scene_parcels_at(0.0)
}
