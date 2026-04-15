use godot::prelude::*;
use godot::classes::{
    RenderingDevice,
    RdUniform,
};
use godot::classes::rendering_device::UniformType;
use super::SplatViewer;

impl SplatViewer {
    pub fn create_uniform_set(&mut self, rd: &mut Gd<RenderingDevice>) {
        let mut uniform = RdUniform::new_gd();

        uniform.set_uniform_type(UniformType::IMAGE);
        uniform.set_binding(0);
        uniform.add_id(self.texture_rid);

        self.uniform_set_rid = rd.uniform_set_create(
            &Array::from_iter([uniform]),
            self.shader_rid,
            0,
        );
    }
}
