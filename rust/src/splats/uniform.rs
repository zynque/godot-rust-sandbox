use godot::prelude::*;
use godot::builtin::PackedByteArray;
use godot::classes::{
    RenderingDevice,
    RdUniform,
};
use godot::classes::rendering_device::UniformType;
use super::SplatViewer;
use super::gpu_splat_data::single_upward_segment_splat_bytes;

const CAMERA_UNIFORM_SIZE_BYTES: u32 = 128;

fn transform3d_to_mat4_cols(transform: Transform3D) -> [f32; 16] {
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

fn projection_to_mat4_cols(projection: Projection) -> [f32; 16] {
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

pub fn camera_uniform_bytes(camera_transform: Transform3D, camera_projection: Projection) -> PackedByteArray {
    let view = camera_transform.affine_inverse();
    let view_floats = transform3d_to_mat4_cols(view);
    let projection_floats = projection_to_mat4_cols(camera_projection);

    let mut bytes = Vec::with_capacity(CAMERA_UNIFORM_SIZE_BYTES as usize);
    for f in view_floats {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    for f in projection_floats {
        bytes.extend_from_slice(&f.to_le_bytes());
    }

    PackedByteArray::from(bytes.as_slice())
}

pub fn update_camera_buffer_raw(
    rd: &mut Gd<RenderingDevice>,
    camera_buffer_rid: Rid,
    data: &PackedByteArray,
) {
    let _ = rd.buffer_update(camera_buffer_rid, 0, CAMERA_UNIFORM_SIZE_BYTES, data);
}

impl SplatViewer {
    pub fn create_splat_buffer(&mut self, rd: &mut Gd<RenderingDevice>) {
        let bytes = single_upward_segment_splat_bytes();
        let packed_bytes = PackedByteArray::from(bytes.as_slice());

        self.splat_buffer_rid = rd.storage_buffer_create_ex(bytes.len() as u32)
            .data(&packed_bytes)
            .done();
    }

    pub fn create_camera_buffer(&mut self, rd: &mut Gd<RenderingDevice>) {
        self.camera_buffer_rid = rd.uniform_buffer_create(CAMERA_UNIFORM_SIZE_BYTES);
    }

    pub fn create_uniform_set(&mut self, rd: &mut Gd<RenderingDevice>) {
        let mut image_uniform = RdUniform::new_gd();

        image_uniform.set_uniform_type(UniformType::IMAGE);
        image_uniform.set_binding(0);
        image_uniform.add_id(self.texture_rid);

        let mut splat_uniform = RdUniform::new_gd();
        splat_uniform.set_uniform_type(UniformType::STORAGE_BUFFER);
        splat_uniform.set_binding(1);
        splat_uniform.add_id(self.splat_buffer_rid);

        let mut camera_uniform = RdUniform::new_gd();
        camera_uniform.set_uniform_type(UniformType::UNIFORM_BUFFER);
        camera_uniform.set_binding(2);
        camera_uniform.add_id(self.camera_buffer_rid);

        self.uniform_set_rid = rd.uniform_set_create(
            &Array::from_iter([image_uniform, splat_uniform, camera_uniform]),
            self.shader_rid,
            0,
        );
    }
}
