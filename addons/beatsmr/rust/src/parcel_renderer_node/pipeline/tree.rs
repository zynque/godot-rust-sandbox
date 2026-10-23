use godot::prelude::*;

use super::geometry::{covariance_from_shape, inverse_covariance_from_shape, ParcelGpu, ParcelShape};

#[derive(Clone, Copy)]
struct TreeSegment {
    origin: Vector3,
    direction: Vector3,
    length: f32,
    thickness: f32,
    depth: u32,
}

fn rotate_about_axis(vector: Vector3, axis: Vector3, angle_radians: f32) -> Vector3 {
    let axis_length_sq = axis.length_squared();
    if axis_length_sq <= f32::EPSILON {
        return vector;
    }

    let axis_normalized = axis / axis_length_sq.sqrt();
    let cos_a = angle_radians.cos();
    let sin_a = angle_radians.sin();
    let dot = vector.dot(axis_normalized);
    let cross = axis_normalized.cross(vector);
    vector * cos_a + cross * sin_a + axis_normalized * dot * (1.0 - cos_a)
}

fn make_tree_parcel(mean: Vector3, direction: Vector3, length: f32, thickness: f32, color: Vector3) -> ParcelGpu {
    let reference = if direction.dot(Vector3::UP).abs() > 0.9 {
        Vector3::RIGHT
    } else {
        Vector3::UP
    };
    let normal = (reference - reference.dot(direction) * direction).normalized();

    let shape = ParcelShape {
        forward: direction,
        normal,
        axis_length: length,
        width: thickness,
        height: thickness * 0.9,
    };

    let covariance = covariance_from_shape(shape);
    let inverse_covariance = inverse_covariance_from_shape(shape);
    ParcelGpu {
        mean: [mean.x, mean.y, mean.z, 0.0],
        covariance,
        inverse_covariance,
        peak_density: 12.0,
        peak_padding: [0.0, 0.0, 0.0],
        color: [color.x, color.y, color.z, 0.0],
    }
}

pub(crate) fn generate_tree_parcels() -> Vec<ParcelGpu> {
    const MAX_TREE_PARCELS: usize = 6;
    const MAX_DEPTH: u32 = 2;

    let mut parcels = Vec::new();
    let mut stack = vec![TreeSegment {
        origin: Vector3::ZERO,
        direction: Vector3::UP,
        length: 2.0,
        thickness: 0.2,
        depth: 0,
    }];

    while let Some(segment) = stack.pop() {
        if parcels.len() >= MAX_TREE_PARCELS {
            break;
        }

        let mean = segment.origin + segment.direction * segment.length * 0.5;
        let green_tone = 0.35 + (segment.depth as f32) * 0.12;
        let trunk_color = Vector3::new(0.15, green_tone.clamp(0.4, 1.0), 0.12);
        let mut parcel = make_tree_parcel(mean, segment.direction, segment.length, segment.thickness, trunk_color);
        parcel.peak_density = 1.0;
        parcels.push(parcel);

        if segment.depth >= MAX_DEPTH {
            continue;
        }

        let branch_origin = segment.origin + segment.direction * segment.length * 0.82;
        let next_length = segment.length * 0.65;
        let next_thickness = segment.thickness * 0.7;
        let spread = 0.55 + segment.depth as f32 * 0.08;

        for branch_index in 0..2 {
            if parcels.len() + stack.len() >= MAX_TREE_PARCELS {
                continue;
            }

            let nudge = if branch_index == 0 { -1.0 } else { 1.0 };
            let twist_axis = Vector3::new(0.0, 0.0, 1.0);
            let branch_dir = rotate_about_axis(segment.direction, twist_axis, spread * nudge);
            let branch_dir = rotate_about_axis(branch_dir, Vector3::new(1.0, 0.0, 0.0), 0.18 + segment.depth as f32 * 0.04);
            let direction = if branch_dir.length_squared() <= f32::EPSILON {
                segment.direction
            } else {
                branch_dir.normalized()
            };
            stack.push(TreeSegment {
                origin: branch_origin,
                direction,
                length: next_length,
                thickness: next_thickness,
                depth: segment.depth + 1,
            });
        }
    }

    godot_print!(
        "tree parcels generated: {} (max: {})",
        parcels.len(),
        MAX_TREE_PARCELS,
    );
    for (index, parcel) in parcels.iter().enumerate() {
        let mean = Vector3::new(parcel.mean[0], parcel.mean[1], parcel.mean[2]);
        let color = Vector3::new(parcel.color[0], parcel.color[1], parcel.color[2]);
        let inv = parcel.inverse_covariance;
        godot_print!(
            "tree parcel[{}]: mean=({}, {}, {}), peak_density={}, color=({}, {}, {}), inverse_covariance=[{:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            index,
            mean.x,
            mean.y,
            mean.z,
            parcel.peak_density,
            color.x,
            color.y,
            color.z,
            inv[0], inv[1], inv[2], inv[3], inv[4], inv[5], inv[6], inv[7], inv[8], inv[9], inv[10], inv[11],
        );
    }

    parcels
}
