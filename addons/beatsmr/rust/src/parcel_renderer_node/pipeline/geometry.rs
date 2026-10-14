use godot::prelude::*;

pub(crate) type GpuVec3 = [f32; 4];
pub(crate) type GpuMat3 = [f32; 12];

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct ParcelGpu {
    pub mean: GpuVec3,
    pub covariance: GpuMat3,
    pub inverse_covariance: GpuMat3,
    pub peak_density: f32,
    pub peak_padding: [f32; 3],
    pub color: GpuVec3,
}

#[derive(Clone, Copy)]
pub(crate) struct ParcelShape {
    pub forward: Vector3,
    pub normal: Vector3,
    pub axis_length: f32,
    pub width: f32,
    pub height: f32,
}

pub(crate) const MAX_PARCELS: usize = 100;
pub(crate) const PARCEL_FLOATS_PER_INSTANCE: usize = 36;

impl ParcelGpu {
    pub(crate) fn as_f32s(&self) -> [f32; PARCEL_FLOATS_PER_INSTANCE] {
        let mut out = [0.0; PARCEL_FLOATS_PER_INSTANCE];
        let mut cursor = 0;

        out[cursor..cursor + self.mean.len()].copy_from_slice(&self.mean);
        cursor += self.mean.len();
        out[cursor..cursor + self.covariance.len()].copy_from_slice(&self.covariance);
        cursor += self.covariance.len();
        out[cursor..cursor + self.inverse_covariance.len()]
            .copy_from_slice(&self.inverse_covariance);
        cursor += self.inverse_covariance.len();
        out[cursor] = self.peak_density;
        cursor += 1;
        out[cursor..cursor + self.peak_padding.len()].copy_from_slice(&self.peak_padding);
        cursor += self.peak_padding.len();
        out[cursor..cursor + self.color.len()].copy_from_slice(&self.color);
        out
    }
}

fn orthogonalize(v1: Vector3, v2: Vector3) -> Vector3 {
    let projection = v1 * v2.dot(v1);
    let result = v2 - projection;
    if result.length_squared() <= f32::EPSILON {
        return Vector3::UP;
    }
    result.normalized()
}

fn vector_component(v: Vector3, axis: usize) -> f32 {
    match axis {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

fn outer_product(u: Vector3, v: Vector3) -> [[f32; 3]; 3] {
    let mut matrix = [[0.0; 3]; 3];
    for row in 0..3 {
        for col in 0..3 {
            matrix[row][col] = vector_component(u, row) * vector_component(v, col);
        }
    }
    matrix
}

fn flatten_mat3(matrix: [[f32; 3]; 3]) -> GpuMat3 {
    let mut out = [0.0; 12];
    for col in 0..3 {
        for row in 0..3 {
            out[col * 4 + row] = matrix[row][col];
        }
    }
    out
}

pub(crate) fn covariance_from_shape(shape: ParcelShape) -> GpuMat3 {
    let forward = shape.forward.normalized();
    let normal = orthogonalize(forward, shape.normal.normalized());
    let right = forward.cross(normal).normalized();

    let l2 = shape.axis_length * shape.axis_length;
    let w2 = shape.width * shape.width;
    let t2 = shape.height * shape.height;

    let mut matrix = [[0.0; 3]; 3];
    for row in 0..3 {
        for col in 0..3 {
            let forward_term = outer_product(forward, forward)[row][col];
            let right_term = outer_product(right, right)[row][col];
            let normal_term = outer_product(normal, normal)[row][col];
            matrix[row][col] = l2 * forward_term + w2 * right_term + t2 * normal_term;
        }
    }

    flatten_mat3(matrix)
}

pub(crate) fn inverse_covariance_from_shape(shape: ParcelShape) -> GpuMat3 {
    let forward = shape.forward.normalized();
    let normal = orthogonalize(forward, shape.normal.normalized());
    let right = forward.cross(normal).normalized();

    let l2 = shape.axis_length * shape.axis_length;
    let w2 = shape.width * shape.width;
    let t2 = shape.height * shape.height;

    let mut matrix = [[0.0; 3]; 3];
    for row in 0..3 {
        for col in 0..3 {
            let forward_term = outer_product(forward, forward)[row][col];
            let right_term = outer_product(right, right)[row][col];
            let normal_term = outer_product(normal, normal)[row][col];
            let inv_l2 = if l2 > 0.0 { 1.0 / l2 } else { 0.0 };
            let inv_w2 = if w2 > 0.0 { 1.0 / w2 } else { 0.0 };
            let inv_t2 = if t2 > 0.0 { 1.0 / t2 } else { 0.0 };
            matrix[row][col] = inv_l2 * forward_term + inv_w2 * right_term + inv_t2 * normal_term;
        }
    }

    flatten_mat3(matrix)
}

pub(crate) fn transform3d_to_mat4_cols(transform: Transform3D) -> [f32; 16] {
    let cols = transform.basis.to_cols();
    let a = cols[0];
    let b = cols[1];
    let c = cols[2];
    let o = transform.origin;

    [
        a.x, a.y, a.z, 0.0,
        b.x, b.y, b.z, 0.0,
        c.x, c.y, c.z, 0.0,
        o.x, o.y, o.z, 1.0,
    ]
}

pub(crate) fn projection_to_mat4_cols(projection: Projection) -> [f32; 16] {
    let x = projection.cols[0];
    let y = projection.cols[1];
    let z = projection.cols[2];
    let w = projection.cols[3];

    [
        x.x, x.y, x.z, x.w,
        y.x, y.y, y.z, y.w,
        z.x, z.y, z.z, z.w,
        w.x, w.y, w.z, w.w,
    ]
}

pub(crate) fn camera_uniform_bytes(camera_transform: Transform3D, camera_projection: Projection) -> PackedByteArray {
    let view = camera_transform.affine_inverse();
    let view_floats = transform3d_to_mat4_cols(view);
    let projection_floats = projection_to_mat4_cols(camera_projection);

    let mut bytes = Vec::with_capacity(32 * std::mem::size_of::<f32>());
    for f in view_floats {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    for f in projection_floats {
        bytes.extend_from_slice(&f.to_le_bytes());
    }

    PackedByteArray::from(bytes.as_slice())
}

pub(crate) fn sample_parcels_for_time(time_seconds: f32) -> [ParcelGpu; 6] {
    let c = (0.8 * time_seconds).cos();

    fn make_parcel(mean: Vector3, shape: ParcelShape, peak_density: f32, color: Vector3) -> ParcelGpu {
        let covariance = covariance_from_shape(shape);
        let inverse_covariance = inverse_covariance_from_shape(shape);
        ParcelGpu {
            mean: [mean.x, mean.y, mean.z, 0.0],
            covariance,
            inverse_covariance,
            peak_density,
            peak_padding: [0.0, 0.0, 0.0],
            color: [color.x, color.y, color.z, 0.0],
        }
    }

    let shape_a = ParcelShape {
        forward: Vector3::BACK,
        normal: Vector3::UP,
        axis_length: 0.3,
        width: 0.3,
        height: 0.02,
    };
    let shape_b = ParcelShape {
        forward: Vector3::BACK,
        normal: Vector3::UP,
        axis_length: 0.3,
        width: 0.3,
        height: 0.02,
    };
    let shape_c = ParcelShape {
        forward: Vector3::BACK,
        normal: Vector3::UP,
        axis_length: 0.3,
        width: 0.02,
        height: 0.05,
    };

    let sample_parcels = [
        make_parcel(Vector3::new(-0.6 * c, 1.0, -3.0), shape_a, 0.8, Vector3::new(0.25, 0.75, 0.35)),
        make_parcel(Vector3::new(0.6 * c, 1.0, -3.0), shape_b, 0.8, Vector3::new(0.25, 0.75, 0.35)),
        make_parcel(Vector3::new(0.0, 1.0, -3.0), shape_c, 0.8, Vector3::new(0.25, 0.75, 0.35)),
    ];

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

    [
        make_parcel(Vector3::new(-4.0, 0.0, -5.0), axis_x, 1.0, Vector3::new(1.0, 0.0, 0.0)),
        make_parcel(Vector3::new(0.0, -4.0, -5.0), axis_y, 1.0, Vector3::new(0.0, 1.0, 0.0)),
        make_parcel(Vector3::new(0.0, 0.0, -7.0), axis_z, 1.0, Vector3::new(0.0, 0.2, 1.0)),
        sample_parcels[0],
        sample_parcels[1],
        sample_parcels[2],
    ]
}
