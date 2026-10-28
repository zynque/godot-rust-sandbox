use godot::prelude::*;

/// Capacity of the parcel storage buffer. Must match `MAX_PARCELS` in
/// `shaders/parcel_renderer/core/constants.glslinc`.
pub(crate) const MAX_PARCELS: usize = 30;

/// A `f32` count of one std430 `Parcel` record: `vec4` mean, `mat3`
/// covariance, `mat3` inverse covariance, `float` peak density plus `vec3`
/// padding, `vec4` color.
pub(crate) const PARCEL_FLOATS_PER_INSTANCE: usize = 36;

pub(crate) fn parcel_bytes(values: &[f32]) -> PackedByteArray {
    let mut bytes = Vec::with_capacity(values.len() * std::mem::size_of::<f32>());
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    PackedByteArray::from(bytes.as_slice())
}
