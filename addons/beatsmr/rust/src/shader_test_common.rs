use godot::classes::{
    RdShaderFile,
    RenderingDevice,
    RenderingServer,
};
use godot::prelude::*;

pub fn create_rendering_device(context_name: &str) -> Option<Gd<RenderingDevice>> {
    let device = RenderingServer::singleton().create_local_rendering_device();
    if device.is_none() {
        godot_warn!("{context_name}: failed to create local RenderingDevice; skipping tests.");
    }
    device
}

pub fn compile_compute_pipeline(
    rd: &mut Gd<RenderingDevice>,
    shader_path: &str,
    context_name: &str,
) -> Option<(Rid, Rid)> {
    let shader_file: Gd<RdShaderFile> = load(shader_path);
    let spirv = shader_file.get_spirv();
    if spirv.is_none() {
        godot_warn!("{context_name}: failed to load shader SPIR-V from '{shader_path}'.");
        return None;
    }

    let shader_rid = rd.shader_create_from_spirv(&spirv.unwrap());
    if shader_rid == Rid::Invalid {
        godot_warn!("{context_name}: failed to create shader '{shader_path}'.");
        return None;
    }

    let pipeline_rid = rd.compute_pipeline_create(shader_rid);
    if pipeline_rid == Rid::Invalid {
        godot_warn!("{context_name}: failed to create compute pipeline for '{shader_path}'.");
        rd.free_rid(shader_rid);
        None
    } else {
        Some((shader_rid, pipeline_rid))
    }
}
