use godot::classes::rendering_device::UniformType;
use godot::classes::{RdShaderFile, RdUniform, RenderingDevice, RenderingServer};
use godot::prelude::*;

/// Compiles one compute shader and dispatches single work-groups against it.
///
/// Knows only about flat float storage buffers: `input` is uploaded to
/// binding 0 and `output_floats` floats are read back from binding 1. The
/// shader and pipeline RIDs are freed when the harness is dropped.
pub struct ComputeHarness {
    rd: Gd<RenderingDevice>,
    pipeline_rid: Rid,
    shader_rid: Rid,
}

impl ComputeHarness {
    /// Builds a local rendering device and a compute pipeline for
    /// `shader_path`. Returns `None` (after warning) if either step fails.
    pub fn new(context: &str, shader_path: &str) -> Option<Self> {
        let mut rd = create_rendering_device(context)?;
        let (shader_rid, pipeline_rid) = compile_compute_pipeline(&mut rd, shader_path, context)?;
        Some(Self {
            rd,
            pipeline_rid,
            shader_rid,
        })
    }

    /// Uploads `input` to binding 0, dispatches one work-group, and returns
    /// `output_floats` floats read back from binding 1.
    pub fn dispatch(&mut self, input: &[f32], output_floats: usize) -> Result<Vec<f32>, String> {
        let mut input_bytes = Vec::with_capacity(input.len() * std::mem::size_of::<f32>());
        for value in input {
            input_bytes.extend_from_slice(&value.to_le_bytes());
        }
        let input_byte_count = input_bytes.len() as u32;
        let output_byte_count = (output_floats * std::mem::size_of::<f32>()) as u32;

        let input_buffer_rid = self
            .rd
            .storage_buffer_create_ex(input_byte_count)
            .data(&PackedByteArray::from(input_bytes.as_slice()))
            .done();
        if input_buffer_rid == Rid::Invalid {
            return Err("failed to create input storage buffer".into());
        }

        let zeroed = PackedByteArray::from(vec![0u8; output_byte_count as usize].as_slice());
        let output_buffer_rid = self
            .rd
            .storage_buffer_create_ex(output_byte_count)
            .data(&zeroed)
            .done();
        if output_buffer_rid == Rid::Invalid {
            self.rd.free_rid(input_buffer_rid);
            return Err("failed to create output storage buffer".into());
        }

        let uniform_set_rid = {
            let mut input_uniform = RdUniform::new_gd();
            input_uniform.set_uniform_type(UniformType::STORAGE_BUFFER);
            input_uniform.set_binding(0);
            input_uniform.add_id(input_buffer_rid);

            let mut output_uniform = RdUniform::new_gd();
            output_uniform.set_uniform_type(UniformType::STORAGE_BUFFER);
            output_uniform.set_binding(1);
            output_uniform.add_id(output_buffer_rid);

            self.rd.uniform_set_create(
                &Array::from_iter([input_uniform, output_uniform]),
                self.shader_rid,
                0,
            )
        };
        if uniform_set_rid == Rid::Invalid {
            self.rd.free_rid(input_buffer_rid);
            self.rd.free_rid(output_buffer_rid);
            return Err("failed to create uniform set".into());
        }

        let list = self.rd.compute_list_begin();
        self.rd
            .compute_list_bind_compute_pipeline(list, self.pipeline_rid);
        self.rd
            .compute_list_bind_uniform_set(list, uniform_set_rid, 0);
        self.rd.compute_list_dispatch(list, 1, 1, 1);
        self.rd.compute_list_end();

        self.rd.submit();
        self.rd.sync();

        let result = self.rd.buffer_get_data(output_buffer_rid).to_vec();

        self.rd.free_rid(uniform_set_rid);
        self.rd.free_rid(input_buffer_rid);
        self.rd.free_rid(output_buffer_rid);

        if result.len() < output_byte_count as usize {
            return Err(format!(
                "short read: expected {} bytes, got {}",
                output_byte_count,
                result.len()
            ));
        }

        let mut values = Vec::with_capacity(output_floats);
        for i in 0..output_floats {
            let offset = i * std::mem::size_of::<f32>();
            let bytes: [u8; 4] = result[offset..offset + 4].try_into().unwrap();
            values.push(f32::from_le_bytes(bytes));
        }

        Ok(values)
    }
}

impl Drop for ComputeHarness {
    fn drop(&mut self) {
        self.rd.free_rid(self.pipeline_rid);
        self.rd.free_rid(self.shader_rid);
    }
}

fn create_rendering_device(context_name: &str) -> Option<Gd<RenderingDevice>> {
    let device = RenderingServer::singleton().create_local_rendering_device();
    if device.is_none() {
        godot_warn!("{context_name}: failed to create local RenderingDevice; skipping tests.");
    }
    device
}

fn compile_compute_pipeline(
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
