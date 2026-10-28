use godot::prelude::*;

pub(super) fn transform3d_to_mat4_cols(transform: Transform3D) -> [f32; 16] {
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

pub(super) fn projection_to_mat4_cols(projection: Projection) -> [f32; 16] {
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

pub(super) fn camera_uniform_bytes(
    camera_transform: Transform3D,
    camera_projection: Projection,
) -> PackedByteArray {
    // Rays are built from the inverse matrices only, so the forward view and
    // projection are not uploaded. The camera transform is itself the inverse
    // of the view matrix.
    let inverse_view_floats = transform3d_to_mat4_cols(camera_transform);
    let inverse_projection_floats = projection_to_mat4_cols(camera_projection.inverse());

    let mut bytes = Vec::with_capacity(32 * std::mem::size_of::<f32>());
    for matrix in [inverse_view_floats, inverse_projection_floats] {
        for f in matrix {
            bytes.extend_from_slice(&f.to_le_bytes());
        }
    }

    PackedByteArray::from(bytes.as_slice())
}
