use godot::classes::rendering_device::{DataFormat, ShaderStage, TextureUsageBits, UniformType};
use godot::classes::{ProjectSettings, RdShaderFile, RdTextureFormat, RdTextureView, RdUniform, RenderingDevice};
use godot::prelude::*;

use super::super::{ParcelRendererNode, SHADER_PATH};

fn collapse_log_lines(value: &str) -> String {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" | ")
}

fn log_shader_source_attempt() {
    let file_path = ProjectSettings::singleton()
        .globalize_path(SHADER_PATH)
        .to_string();
    let source = std::fs::read_to_string(&file_path).unwrap_or_else(|error| {
        format!("failed to read shader source from '{file_path}': {error}")
    });

    let compact = collapse_log_lines(&source);
    godot_warn!(
        "ParcelRendererNode: compiling shader '{}'. Source preview: {}",
        SHADER_PATH,
        compact
    );
}

pub(super) fn destroy_resources(node: &mut ParcelRendererNode, rd: &mut Gd<RenderingDevice>) {
    if node.uniform_set_rid != Rid::Invalid {
        rd.free_rid(node.uniform_set_rid);
        node.uniform_set_rid = Rid::Invalid;
    }
    if node.pipeline_rid != Rid::Invalid {
        rd.free_rid(node.pipeline_rid);
        node.pipeline_rid = Rid::Invalid;
    }
    if node.shader_rid != Rid::Invalid {
        rd.free_rid(node.shader_rid);
        node.shader_rid = Rid::Invalid;
    }
    if node.texture_rid != Rid::Invalid {
        rd.free_rid(node.texture_rid);
        node.texture_rid = Rid::Invalid;
    }
    if node.time_buffer_rid != Rid::Invalid {
        rd.free_rid(node.time_buffer_rid);
        node.time_buffer_rid = Rid::Invalid;
    }
    if node.camera_buffer_rid != Rid::Invalid {
        rd.free_rid(node.camera_buffer_rid);
        node.camera_buffer_rid = Rid::Invalid;
    }
    if node.parcel_buffer_rid != Rid::Invalid {
        rd.free_rid(node.parcel_buffer_rid);
        node.parcel_buffer_rid = Rid::Invalid;
    }
}

pub(super) fn create_texture(node: &mut ParcelRendererNode, rd: &mut Gd<RenderingDevice>) {
    let usage = TextureUsageBits::STORAGE_BIT | TextureUsageBits::SAMPLING_BIT;
    let data_format = DataFormat::R32G32B32A32_SFLOAT;

    if !rd.texture_is_format_supported_for_usage(data_format, usage) {
        node.texture_rid = Rid::Invalid;
        godot_warn!(
            "ParcelRendererNode: R32G32B32A32_SFLOAT not supported for STORAGE|SAMPLING usage."
        );
        return;
    }

    let mut format = RdTextureFormat::new_gd();
    format.set_width(node.width);
    format.set_height(node.height);
    format.set_format(data_format);
    format.set_usage_bits(usage);

    let view = RdTextureView::new_gd();
    node.texture_rid = rd.texture_create(&format, &view);
    if node.texture_rid != Rid::Invalid {
        return;
    }

    node.texture_rid = Rid::Invalid;
    godot_warn!("ParcelRendererNode: failed to create RGBA32F output texture.");
}

fn create_time_buffer(node: &mut ParcelRendererNode, rd: &mut Gd<RenderingDevice>) {
    // vec4 time + vec4 scene params.
    node.time_buffer_rid = rd.uniform_buffer_create(32);
    if node.time_buffer_rid == Rid::Invalid {
        godot_warn!("ParcelRendererNode: failed to create iTime uniform buffer.");
    }
}

fn create_camera_buffer(node: &mut ParcelRendererNode, rd: &mut Gd<RenderingDevice>) {
    // Inverse view and inverse projection.
    node.camera_buffer_rid = rd.uniform_buffer_create(128);
    if node.camera_buffer_rid == Rid::Invalid {
        godot_warn!("ParcelRendererNode: failed to create camera uniform buffer.");
    }
}

fn create_parcel_buffer(node: &mut ParcelRendererNode, rd: &mut Gd<RenderingDevice>) {
    let byte_count = ParcelRendererNode::parcel_buffer_size_bytes();
    let initial_data = PackedByteArray::from(vec![0u8; byte_count as usize].as_slice());
    node.parcel_buffer_rid = rd.storage_buffer_create_ex(byte_count).data(&initial_data).done();
    if node.parcel_buffer_rid == Rid::Invalid {
        godot_warn!("ParcelRendererNode: failed to create parcel storage buffer.");
    }
}

pub(super) fn recreate_resources(node: &mut ParcelRendererNode, rd: &mut Gd<RenderingDevice>) {
    destroy_resources(node, rd);
    create_texture(node, rd);
    create_pipeline_and_uniforms(node, rd);
}

pub(super) fn create_pipeline_and_uniforms(node: &mut ParcelRendererNode, rd: &mut Gd<RenderingDevice>) {
    if node.texture_rid == Rid::Invalid {
        godot_warn!("ParcelRendererNode: output texture RID is invalid.");
        return;
    }

    create_time_buffer(node, rd);
    if node.time_buffer_rid == Rid::Invalid {
        return;
    }

    create_camera_buffer(node, rd);
    if node.camera_buffer_rid == Rid::Invalid {
        return;
    }

    create_parcel_buffer(node, rd);
    if node.parcel_buffer_rid == Rid::Invalid {
        return;
    }

    let shader_file: Gd<RdShaderFile> = load(SHADER_PATH);
    if !shader_file.is_instance_valid() {
        godot_warn!("ParcelRendererNode: failed to load shader file {}", SHADER_PATH);
        return;
    }

    let Some(spirv) = shader_file.get_spirv() else {
        godot_warn!("ParcelRendererNode: failed to load SPIR-V from {}", SHADER_PATH);
        log_shader_source_attempt();
        return;
    };

    let bytecode = spirv.get_stage_bytecode(ShaderStage::COMPUTE);
    let byte_count = bytecode.len();
    let compile_error = spirv.get_stage_compile_error(ShaderStage::COMPUTE);

    if !compile_error.is_empty() {
        let compact_error = collapse_log_lines(&compile_error.to_string());
        godot_error!(
            "ParcelRendererNode: shader compile error for {} (compute stage): {}",
            SHADER_PATH,
            compact_error,
        );
    }

    godot_print!(
        "ParcelRendererNode: creating shader from SPIR-V bytes for {} ({} bytes).",
        SHADER_PATH,
        byte_count,
    );

    node.shader_rid = rd.shader_create_from_spirv(&spirv);
    if node.shader_rid == Rid::Invalid {
        godot_error!(
            "ParcelRendererNode: shader creation failed for {}. This usually means the bytecode or included shaders are invalid; shader source dump follows.",
            SHADER_PATH,
        );
        if !compile_error.is_empty() {
            let compact_error = collapse_log_lines(&compile_error.to_string());
            godot_error!("ParcelRendererNode: shader compile details: {}", compact_error);
        }
        log_shader_source_attempt();
        return;
    }

    node.pipeline_rid = rd.compute_pipeline_create(node.shader_rid);
    if node.pipeline_rid == Rid::Invalid {
        godot_warn!("ParcelRendererNode: failed to create compute pipeline RID.");
        return;
    }

    let mut output_image = RdUniform::new_gd();
    output_image.set_uniform_type(UniformType::IMAGE);
    output_image.set_binding(0);
    output_image.add_id(node.texture_rid);

    let mut time_uniform = RdUniform::new_gd();
    time_uniform.set_uniform_type(UniformType::UNIFORM_BUFFER);
    time_uniform.set_binding(1);
    time_uniform.add_id(node.time_buffer_rid);

    let mut camera_uniform = RdUniform::new_gd();
    camera_uniform.set_uniform_type(UniformType::UNIFORM_BUFFER);
    camera_uniform.set_binding(2);
    camera_uniform.add_id(node.camera_buffer_rid);

    let mut parcel_uniform = RdUniform::new_gd();
    parcel_uniform.set_uniform_type(UniformType::STORAGE_BUFFER);
    parcel_uniform.set_binding(3);
    parcel_uniform.add_id(node.parcel_buffer_rid);

    node.uniform_set_rid = rd.uniform_set_create(
        &Array::from_iter([output_image, time_uniform, camera_uniform, parcel_uniform]),
        node.shader_rid,
        0,
    );
    if node.uniform_set_rid == Rid::Invalid {
        godot_warn!(
            "ParcelRendererNode: failed to create uniform set (set=0, binding=0 image, binding=1 iTime, binding=2 camera, binding=3 parcels)."
        );
    }
}
