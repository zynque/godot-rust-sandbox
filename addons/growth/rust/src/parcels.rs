//! Parcel records and the shape-to-covariance math they need.
//!
//! A packed record matches the std430 `Parcel` buffer declared in
//! `addons/beatsmr/shaders/parcel_renderer/core/structs.glslinc`:
//! `vec4 mean`, `mat3 covariance`, `mat3 inverse_covariance`,
//! `float peak_density` + `vec3` padding, `vec4 color`.

use godot::builtin::Vector3;

/// Number of `f32` values in one packed parcel record.
pub const PARCEL_FLOATS: usize = 36;

const MAT3_FLOATS: usize = 12;
const MEAN_FLOATS: usize = 4;

/// Ellipsoid axes of a parcel, measured in standard deviations.
#[derive(Clone, Copy, Debug)]
pub struct ParcelShape {
    /// Direction the long axis points in.
    pub forward: Vector3,
    /// Direction the thin axis points in. Orthogonalized against `forward`.
    pub normal: Vector3,
    /// Length of the long axis.
    pub axis_length: f32,
    /// Width across `forward.cross(normal)`.
    pub width: f32,
    /// Thickness along `normal`.
    pub height: f32,
}

/// A gaussian ellipsoid of matter with a material color.
#[derive(Clone, Copy, Debug)]
pub struct Parcel {
    pub mean: Vector3,
    pub shape: ParcelShape,
    pub peak_density: f32,
    pub color: Vector3,
}

impl Parcel {
    pub fn new(mean: Vector3, shape: ParcelShape, peak_density: f32, color: Vector3) -> Self {
        Self {
            mean,
            shape,
            peak_density,
            color,
        }
    }

    pub fn covariance(&self) -> [f32; MAT3_FLOATS] {
        covariance_matrix(self.shape, false)
    }

    pub fn inverse_covariance(&self) -> [f32; MAT3_FLOATS] {
        covariance_matrix(self.shape, true)
    }

    /// Packs the parcel into the layout the renderer uploads to the GPU.
    pub fn as_f32s(&self) -> [f32; PARCEL_FLOATS] {
        let mut out = [0.0; PARCEL_FLOATS];
        out[0..3].copy_from_slice(&[self.mean.x, self.mean.y, self.mean.z]);
        out[MEAN_FLOATS..MEAN_FLOATS + MAT3_FLOATS].copy_from_slice(&self.covariance());
        out[MEAN_FLOATS + MAT3_FLOATS..MEAN_FLOATS + 2 * MAT3_FLOATS]
            .copy_from_slice(&self.inverse_covariance());
        out[MEAN_FLOATS + 2 * MAT3_FLOATS] = self.peak_density;
        out[MEAN_FLOATS + 2 * MAT3_FLOATS + 4..MEAN_FLOATS + 2 * MAT3_FLOATS + 7]
            .copy_from_slice(&[self.color.x, self.color.y, self.color.z]);
        out
    }
}

fn orthogonalize(v1: Vector3, v2: Vector3) -> Vector3 {
    let v1_norm = if v1.length_squared() <= f32::EPSILON {
        Vector3::UP
    } else {
        v1.normalized()
    };
    let projection = v1_norm * v2.dot(v1_norm);
    let result = v2 - projection;
    if result.length_squared() > f32::EPSILON {
        return result.normalized();
    }

    let fallback = if v1_norm.x.abs() < 0.9 {
        Vector3::RIGHT
    } else if v1_norm.y.abs() < 0.9 {
        Vector3::UP
    } else {
        Vector3::BACK
    };

    let fallback_result = fallback - fallback.dot(v1_norm) * v1_norm;
    if fallback_result.length_squared() > f32::EPSILON {
        fallback_result.normalized()
    } else {
        Vector3::UP
    }
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
    for (row, cells) in matrix.iter_mut().enumerate() {
        for (col, cell) in cells.iter_mut().enumerate() {
            *cell = vector_component(u, row) * vector_component(v, col);
        }
    }
    matrix
}

fn flatten_mat3(matrix: [[f32; 3]; 3]) -> [f32; MAT3_FLOATS] {
    let mut out = [0.0; MAT3_FLOATS];
    for col in 0..3 {
        for row in 0..3 {
            out[col * 4 + row] = matrix[row][col];
        }
    }
    out
}

/// Builds the covariance (or its inverse) from the parcel's ellipsoid axes.
fn covariance_matrix(shape: ParcelShape, inverse: bool) -> [f32; MAT3_FLOATS] {
    let forward = shape.forward.normalized();
    let normal = orthogonalize(forward, shape.normal.normalized());
    let right = forward.cross(normal).normalized();

    let mut variances = [
        shape.axis_length * shape.axis_length,
        shape.width * shape.width,
        shape.height * shape.height,
    ];
    if inverse {
        variances = variances.map(|variance| {
            if variance > 0.0 { 1.0 / variance } else { 0.0 }
        });
    }

    let mut matrix = [[0.0; 3]; 3];
    for (basis, variance) in [
        (forward, variances[0]),
        (right, variances[1]),
        (normal, variances[2]),
    ] {
        let term = outer_product(basis, basis);
        for (row, cells) in matrix.iter_mut().enumerate() {
            for (col, cell) in cells.iter_mut().enumerate() {
                *cell += variance * term[row][col];
            }
        }
    }

    flatten_mat3(matrix)
}
