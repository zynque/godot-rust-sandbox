use godot::prelude::*;
use godot::classes::{RenderingServer, RdShaderSource, RdUniform};
use godot::classes::rendering_device::{ShaderStage, UniformType};
use godot::classes::FileAccess;

/// A single dot produced by the GPU compute shader.
pub struct GpuDot {
    pub position: Vector2,
    pub radius: f32,
    pub color: Color,
}

/// Load compute shader from bindless_compute.gdshader, stripping the #[compute] directive.
/// Returns the GLSL source or None if the file cannot be read.
fn load_shader_source() -> Option<String> {
    let path = GString::from("res://bindless_compute.gdshader");
    let file = FileAccess::open(&path, godot::classes::file_access::ModeFlags::READ)?;
    
    let content_gstring = file.get_as_text();
    let content = String::from(content_gstring);
    
    // Strip the optional #[compute] directive at the start (Godot shader annotation)
    let glsl: String = content
        .split('\n')
        .skip_while(|line| line.trim().starts_with("#[compute]"))
        .collect::<Vec<&str>>()
        .join("\n");
    
    Some(glsl)
}

/// Run a one-shot compute shader that generates background dots.
/// Returns the dots on success, or an empty vec on failure.
pub fn compute_background_dots(count: u32) -> Vec<GpuDot> {
    // --- 1. Local RenderingDevice (requires Forward+ or Mobile) ---
    let rs = RenderingServer::singleton();
    let Some(mut rd) = rs.create_local_rendering_device() else {
        godot_error!("Bindless: create_local_rendering_device() returned None \
                      (requires Forward+ or Mobile renderer, not Compatibility)");
        return Vec::new();
    };

    // --- 2. Load and compile GLSL → SPIR-V at runtime ---
    let Some(glsl_source) = load_shader_source() else {
        godot_error!("Bindless: failed to load shader from res://bindless_compute.gdshader");
        return Vec::new();
    };

    let mut source = RdShaderSource::new_gd();
    source.set_stage_source(ShaderStage::COMPUTE, &GString::from(glsl_source.as_str()));

    let Some(spirv) = rd.shader_compile_spirv_from_source(&source) else {
        godot_error!("Bindless: shader_compile_spirv_from_source returned None");
        return Vec::new();
    };

    let compile_err = spirv.get_stage_compile_error(ShaderStage::COMPUTE);
    if !compile_err.is_empty() {
        godot_error!("Bindless: compute shader compile error:\n{}", compile_err);
        return Vec::new();
    }

    let shader = rd.shader_create_from_spirv(&spirv);
    if !shader.is_valid() {
        godot_error!("Bindless: shader_create_from_spirv failed");
        return Vec::new();
    }

    // --- 3. Compute pipeline ---
    let pipeline = rd.compute_pipeline_create(shader);
    if !pipeline.is_valid() {
        godot_error!("Bindless: compute_pipeline_create failed");
        rd.free_rid(shader);
        return Vec::new();
    }

    // --- 4. Storage buffers (vec4 = 16 bytes per element) ---
    let buf_bytes = count * 16;
    let instance_buf = rd.storage_buffer_create(buf_bytes);
    let color_buf    = rd.storage_buffer_create(buf_bytes);
    if !instance_buf.is_valid() || !color_buf.is_valid() {
        godot_error!("Bindless: storage_buffer_create failed");
        rd.free_rid(pipeline);
        rd.free_rid(shader);
        return Vec::new();
    }

    // --- 5. Uniform set (binding 0 = positions, binding 1 = colours) ---
    let mut u0 = RdUniform::new_gd();
    u0.set_uniform_type(UniformType::STORAGE_BUFFER);
    u0.set_binding(0);
    u0.add_id(instance_buf);

    let mut u1 = RdUniform::new_gd();
    u1.set_uniform_type(UniformType::STORAGE_BUFFER);
    u1.set_binding(1);
    u1.add_id(color_buf);

    let mut uniforms: Array<Gd<RdUniform>> = Array::new();
    uniforms.push(&u0);
    uniforms.push(&u1);

    let uniform_set = rd.uniform_set_create(&uniforms, shader, 0);

    // --- 6. Dispatch compute ---
    let list = rd.compute_list_begin();
    rd.compute_list_bind_compute_pipeline(list, pipeline);
    rd.compute_list_bind_uniform_set(list, uniform_set, 0);
    let workgroups = (count + 63) / 64;
    rd.compute_list_dispatch(list, workgroups, 1, 1);
    rd.compute_list_end();

    rd.submit();
    rd.sync(); // block until done

    // --- 7. Read-back ---
    let pos_data = rd.buffer_get_data(instance_buf);
    let col_data = rd.buffer_get_data(color_buf);

    // --- 8. Cleanup GPU resources ---
    rd.free_rid(instance_buf);
    rd.free_rid(color_buf);
    rd.free_rid(uniform_set);
    rd.free_rid(pipeline);
    rd.free_rid(shader);

    // --- 9. Parse raw bytes → GpuDot ---
    let mut dots = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        let off = i * 16; // 4 floats × 4 bytes
        if off + 16 > pos_data.len() || off + 16 > col_data.len() {
            break;
        }

        let px = read_f32(&pos_data, off);
        let py = read_f32(&pos_data, off + 4);
        let pr = read_f32(&pos_data, off + 8);

        let cr = read_f32(&col_data, off);
        let cg = read_f32(&col_data, off + 4);
        let cb = read_f32(&col_data, off + 8);
        let ca = read_f32(&col_data, off + 12);

        dots.push(GpuDot {
            position: Vector2::new(px, py),
            radius: pr,
            color: Color::from_rgba(cr, cg, cb, ca),
        });
    }

    godot_print!("Bindless compute: {} background dots generated on GPU ✓", dots.len());
    dots
}

/// Read a little-endian f32 from a PackedByteArray at the given byte offset.
fn read_f32(data: &PackedByteArray, off: usize) -> f32 {
    f32::from_le_bytes([
        data[off],
        data[off + 1],
        data[off + 2],
        data[off + 3],
    ])
}
