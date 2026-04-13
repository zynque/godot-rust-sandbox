use godot::prelude::*;
use godot::classes::{RenderingServer, RdPipelineColorBlendState, RdPipelineColorBlendStateAttachment, RdPipelineDepthStencilState, RdPipelineMultisampleState, RdPipelineRasterizationState, RdShaderSource, RdUniform, RdVertexAttribute};
use godot::classes::rendering_device::{BlendFactor, BlendOperation, DataFormat, PolygonCullMode, RenderPrimitive, ShaderStage, UniformType, VertexFrequency};

const VERTEX_GLSL: &str = r"#version 450

layout(location = 0) in vec2 in_vertex;
layout(location = 0) out vec2 v_uv;
layout(location = 1) out vec4 v_color_density;

layout(set = 0, binding = 0, std430) readonly buffer SplatBuffer {
    vec4 data[];
};

layout(push_constant, std430) uniform PushConstants {
    mat4 view_proj;
} pc;

void main() {
    int id = gl_InstanceIndex;
    int base = id * 5;

    vec3 position = data[base + 0].xyz;
    vec4 color_density = data[base + 4];

    vec3 right = vec3(1.0, 0.0, 0.0);
    vec3 up = vec3(0.0, 1.0, 0.0);
    float scale = 0.06;

    vec3 world_pos =
        position +
        right * in_vertex.x * scale +
        up * in_vertex.y * scale;

    gl_Position = pc.view_proj * vec4(world_pos, 1.0);
    v_uv = in_vertex * 0.5 + 0.5;
    v_color_density = color_density;
}
";

const FRAGMENT_GLSL: &str = r"#version 450

layout(location = 0) in vec2 v_uv;
layout(location = 1) in vec4 v_color_density;
layout(location = 0) out vec4 out_color;

void main() {
    vec2 uv = v_uv * 2.0 - 1.0;

    float r2 = dot(uv, uv);
    float sigma = 0.5;
    float weight = exp(-r2 / (2.0 * sigma * sigma));

    if (weight < 0.01) {
        discard;
    }

    vec3 color = v_color_density.rgb;
    float density = v_color_density.a;
    out_color = vec4(color * weight * density, weight * density);
}
";

#[repr(C)]
pub struct GPUSplat {
    pub position: [f32; 3],
    pub density: f32,
    pub cov0: [f32; 3],
    pub cov1: [f32; 3],
    pub cov2: [f32; 3],
    pub color: [f32; 3],
    pub pad: f32,
}

pub struct DemoSplat2D {
    pub position: Vector2,
    pub radius: f32,
    pub color: Color,
}

pub fn build_center_demo_splats(center: Vector2) -> Vec<DemoSplat2D> {
    let offsets = [
        Vector2::new(0.0, 0.0),
        Vector2::new(-64.0, -18.0),
        Vector2::new(68.0, -12.0),
        Vector2::new(-26.0, 52.0),
        Vector2::new(34.0, 44.0),
    ];

    let radii = [42.0, 30.0, 34.0, 26.0, 22.0];
    let colors = [
        Color::from_rgba(0.94, 0.53, 0.30, 0.38),
        Color::from_rgba(0.22, 0.78, 0.92, 0.34),
        Color::from_rgba(0.96, 0.84, 0.34, 0.30),
        Color::from_rgba(0.44, 0.85, 0.45, 0.28),
        Color::from_rgba(0.94, 0.40, 0.62, 0.30),
    ];

    let mut splats = Vec::with_capacity(offsets.len());
    for i in 0..offsets.len() {
        splats.push(DemoSplat2D {
            position: center + offsets[i],
            radius: radii[i],
            color: colors[i],
        });
    }

    splats
}

pub fn render_gaussian_splats(instance_count: u32) -> bool {
    let rs = RenderingServer::singleton();
    let Some(mut rd) = rs.create_local_rendering_device() else {
        godot_error!("Bindless RD: create_local_rendering_device() returned None");
        return false;
    };

    let mut shader_src = RdShaderSource::new_gd();
    shader_src.set_stage_source(ShaderStage::VERTEX, VERTEX_GLSL);
    shader_src.set_stage_source(ShaderStage::FRAGMENT, FRAGMENT_GLSL);

    let Some(spirv) = rd.shader_compile_spirv_from_source(&shader_src) else {
        godot_error!("Bindless RD: shader_compile_spirv_from_source returned None");
        return false;
    };

    let vertex_err = spirv.get_stage_compile_error(ShaderStage::VERTEX);
    if !vertex_err.is_empty() {
        godot_error!("Bindless RD: vertex compile error:\n{}", vertex_err);
        return false;
    }
    let fragment_err = spirv.get_stage_compile_error(ShaderStage::FRAGMENT);
    if !fragment_err.is_empty() {
        godot_error!("Bindless RD: fragment compile error:\n{}", fragment_err);
        return false;
    }

    let shader = rd.shader_create_from_spirv(&spirv);
    if !shader.is_valid() {
        godot_error!("Bindless RD: shader_create_from_spirv failed");
        return false;
    }

    let splats = build_demo_splats(instance_count);
    let splat_bytes = pack_splats_std430_words(&splats);
    let splat_buffer = rd.storage_buffer_create(splat_bytes.len() as u32);
    if !splat_buffer.is_valid() {
        godot_error!("Bindless RD: storage_buffer_create failed");
        rd.free_rid(shader);
        return false;
    }
    let update_err = rd.buffer_update(splat_buffer, 0, splat_bytes.len() as u32, &splat_bytes);
    if update_err != godot::global::Error::OK {
        godot_error!("Bindless RD: buffer_update failed with {:?}", update_err);
        rd.free_rid(splat_buffer);
        rd.free_rid(shader);
        return false;
    }

    let quad_bytes = pack_quad_vertices();
    let vertex_buffer = rd.vertex_buffer_create(quad_bytes.len() as u32);
    if !vertex_buffer.is_valid() {
        godot_error!("Bindless RD: vertex_buffer_create failed");
        rd.free_rid(splat_buffer);
        rd.free_rid(shader);
        return false;
    }
    let vbuf_err = rd.buffer_update(vertex_buffer, 0, quad_bytes.len() as u32, &quad_bytes);
    if vbuf_err != godot::global::Error::OK {
        godot_error!("Bindless RD: vertex buffer update failed with {:?}", vbuf_err);
        rd.free_rid(vertex_buffer);
        rd.free_rid(splat_buffer);
        rd.free_rid(shader);
        return false;
    }

    let mut attr = RdVertexAttribute::new_gd();
    attr.set_location(0);
    attr.set_offset(0);
    attr.set_format(DataFormat::R32G32_SFLOAT);
    attr.set_stride(8);
    attr.set_frequency(VertexFrequency::VERTEX);

    let mut attrs: Array<Gd<RdVertexAttribute>> = Array::new();
    attrs.push(&attr);
    let vertex_format = rd.vertex_format_create(&attrs);

    let mut src_buffers: Array<Rid> = Array::new();
    src_buffers.push(vertex_buffer);
    let vertex_array = rd.vertex_array_create(4, vertex_format, &src_buffers);
    if !vertex_array.is_valid() {
        godot_error!("Bindless RD: vertex_array_create failed");
        rd.free_rid(vertex_buffer);
        rd.free_rid(splat_buffer);
        rd.free_rid(shader);
        return false;
    }

    let mut u0 = RdUniform::new_gd();
    u0.set_uniform_type(UniformType::STORAGE_BUFFER);
    u0.set_binding(0);
    u0.add_id(splat_buffer);
    let mut uniforms: Array<Gd<RdUniform>> = Array::new();
    uniforms.push(&u0);
    let uniform_set = rd.uniform_set_create(&uniforms, shader, 0);

    let mut raster = RdPipelineRasterizationState::new_gd();
    raster.set_cull_mode(PolygonCullMode::DISABLED);

    let multisample = RdPipelineMultisampleState::new_gd();

    let mut depth = RdPipelineDepthStencilState::new_gd();
    depth.set_enable_depth_test(false);
    depth.set_enable_depth_write(false);

    let mut blend_attachment = RdPipelineColorBlendStateAttachment::new_gd();
    blend_attachment.set_enable_blend(true);
    blend_attachment.set_src_color_blend_factor(BlendFactor::ONE);
    blend_attachment.set_dst_color_blend_factor(BlendFactor::ONE);
    blend_attachment.set_color_blend_op(BlendOperation::ADD);
    blend_attachment.set_src_alpha_blend_factor(BlendFactor::ONE);
    blend_attachment.set_dst_alpha_blend_factor(BlendFactor::ONE);
    blend_attachment.set_alpha_blend_op(BlendOperation::ADD);

    let mut blend_attachments: Array<Gd<RdPipelineColorBlendStateAttachment>> = Array::new();
    blend_attachments.push(&blend_attachment);

    let mut blend_state = RdPipelineColorBlendState::new_gd();
    blend_state.set_attachments(&blend_attachments);

    let framebuffer_format = rd.screen_get_framebuffer_format();
    let pipeline = rd.render_pipeline_create(
        shader,
        framebuffer_format,
        vertex_format,
        RenderPrimitive::TRIANGLE_STRIPS,
        &raster,
        &multisample,
        &depth,
        &blend_state,
    );
    if !pipeline.is_valid() {
        godot_error!("Bindless RD: render_pipeline_create failed");
        rd.free_rid(uniform_set);
        rd.free_rid(vertex_array);
        rd.free_rid(vertex_buffer);
        rd.free_rid(splat_buffer);
        rd.free_rid(shader);
        return false;
    }

    let draw_list = rd.draw_list_begin_for_screen();
    rd.draw_list_bind_render_pipeline(draw_list, pipeline);
    rd.draw_list_bind_uniform_set(draw_list, uniform_set, 0);
    rd.draw_list_bind_vertex_array(draw_list, vertex_array);
    let push_constants = identity_mat4_push_constants();
    rd.draw_list_set_push_constant(draw_list, &push_constants, 64);
    rd.draw_list_draw(draw_list, false, instance_count);
    rd.draw_list_end();

    rd.submit();
    rd.sync();

    rd.free_rid(uniform_set);
    rd.free_rid(vertex_array);
    rd.free_rid(vertex_buffer);
    rd.free_rid(splat_buffer);
    rd.free_rid(pipeline);
    rd.free_rid(shader);

    godot_print!("Bindless RD: rendered {} gaussian splats", instance_count);
    true
}

fn build_demo_splats(count: u32) -> Vec<GPUSplat> {
    let mut splats = Vec::with_capacity(count as usize);
    for i in 0..count {
        let fi = i as f32;
        let angle = fi * 0.11;
        let r = 0.15 + (fi * 0.017).sin().abs() * 0.65;
        let x = angle.cos() * r;
        let y = angle.sin() * r;
        let color = [
            0.2 + 0.6 * (fi * 0.37).sin().abs(),
            0.3 + 0.5 * (fi * 0.53).sin().abs(),
            0.4 + 0.4 * (fi * 0.71).sin().abs(),
        ];

        splats.push(GPUSplat {
            position: [x, y, 0.0],
            density: 0.8,
            cov0: [1.0, 0.0, 0.0],
            cov1: [0.0, 1.0, 0.0],
            cov2: [0.0, 0.0, 1.0],
            color,
            pad: 1.0,
        });
    }
    splats
}

fn pack_splats_std430_words(splats: &[GPUSplat]) -> PackedByteArray {
    let mut out = PackedByteArray::new();
    for s in splats {
        append_vec4(&mut out, [s.position[0], s.position[1], s.position[2], s.density]);
        append_vec4(&mut out, [s.cov0[0], s.cov0[1], s.cov0[2], 0.0]);
        append_vec4(&mut out, [s.cov1[0], s.cov1[1], s.cov1[2], 0.0]);
        append_vec4(&mut out, [s.cov2[0], s.cov2[1], s.cov2[2], 0.0]);
        append_vec4(&mut out, [s.color[0], s.color[1], s.color[2], s.pad]);
    }
    out
}

fn pack_quad_vertices() -> PackedByteArray {
    let mut out = PackedByteArray::new();
    let verts = [
        [-1.0_f32, -1.0_f32],
        [ 1.0_f32, -1.0_f32],
        [-1.0_f32,  1.0_f32],
        [ 1.0_f32,  1.0_f32],
    ];
    for v in verts {
        append_f32(&mut out, v[0]);
        append_f32(&mut out, v[1]);
    }
    out
}

fn identity_mat4_push_constants() -> PackedByteArray {
    let mut out = PackedByteArray::new();
    let m = [
        1.0_f32, 0.0, 0.0, 0.0,
        0.0, 1.0_f32, 0.0, 0.0,
        0.0, 0.0, 1.0_f32, 0.0,
        0.0, 0.0, 0.0, 1.0_f32,
    ];
    for x in m {
        append_f32(&mut out, x);
    }
    out
}

fn append_vec4(dst: &mut PackedByteArray, v: [f32; 4]) {
    append_f32(dst, v[0]);
    append_f32(dst, v[1]);
    append_f32(dst, v[2]);
    append_f32(dst, v[3]);
}

fn append_f32(dst: &mut PackedByteArray, value: f32) {
    let bytes = value.to_le_bytes();
    dst.push(bytes[0]);
    dst.push(bytes[1]);
    dst.push(bytes[2]);
    dst.push(bytes[3]);
}
