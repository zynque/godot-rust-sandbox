use godot::prelude::*;
use godot::classes::{
    RenderingDevice,
    RdShaderFile,
};
use super::SplatViewer;

impl SplatViewer {
    pub fn create_pipeline(&mut self, rd: &mut Gd<RenderingDevice>) {
        let shader: Gd<RdShaderFile> = load("res://compute_shader.glsl");

        let spirv = shader.get_spirv();
        spirv.map(|spirv| {
            self.shader_rid = rd.shader_create_from_spirv(&spirv);
            self.pipeline_rid = rd.compute_pipeline_create(self.shader_rid);
        });
    }
}
