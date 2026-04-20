use godot::prelude::*;
use godot::builtin::PackedByteArray;
use godot::classes::{
    RenderingDevice,
    RdUniform,
};
use godot::classes::rendering_device::UniformType;
use super::SplatViewer;
use super::gpu_splat_data::single_upward_segment_splat_bytes;

impl SplatViewer {
    pub fn create_splat_buffer(&mut self, rd: &mut Gd<RenderingDevice>) {
        let bytes = single_upward_segment_splat_bytes();
        let packed_bytes = PackedByteArray::from(bytes.as_slice());

        self.splat_buffer_rid = rd.storage_buffer_create_ex(bytes.len() as u32)
            .data(&packed_bytes)
            .done();
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

        self.uniform_set_rid = rd.uniform_set_create(
            &Array::from_iter([image_uniform, splat_uniform]),
            self.shader_rid,
            0,
        );
    }
}
