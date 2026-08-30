use godot::prelude::*;
use godot::classes::{
    Engine,
    RdShaderFile,
    RdUniform,
    RenderingDevice,
    RenderingServer,
};
use godot::classes::rendering_device::UniformType;

// ---------------------------------------------------------------------------
// SortTester
//
// A #[tool] Node that, when it enters the scene tree inside the Godot editor,
// automatically:
//   1. Compiles addons/beatsmr/shaders/sort_test.glsl as a compute shader.
//   2. Uploads a small test array via a storage buffer.
//   3. Dispatches one work-group.
//   4. Reads the buffer back and checks whether the keys are sorted.
//   5. Prints a PASS / FAIL report to the Godot Output panel.
//
// This establishes the pattern for all future shader-testing tools under
// addons/beatsmr.
// ---------------------------------------------------------------------------

// Each test case is (label, input slice).  The expected output is always the
// ascending-sorted version of the input.
const TEST_CASES: &[(&str, &[f32])] = &[
    ("empty",           &[]),
    ("single",          &[42.0]),
    ("already sorted",  &[1.0, 2.0, 3.0, 4.0, 5.0]),
    ("reverse order",   &[9.0, 7.0, 5.0, 3.0, 1.0]),
];

// ---------------------------------------------------------------------------
// GodotClass
// ---------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(tool, base=Node)]
pub struct SortTester {
    base: Base<Node>,
}

#[godot_api]
impl INode for SortTester {
    fn init(base: Base<Node>) -> Self {
        Self { base }
    }

    fn ready(&mut self) {
        if !Engine::singleton().is_editor_hint() {
            return;
        }

        let Some(mut rd) = RenderingServer::singleton().get_rendering_device() else {
            godot_warn!("SortTester: no RenderingDevice available – skipping tests.");
            return;
        };

        let Some((shader_rid, pipeline_rid)) = compile_pipeline(&mut rd) else {
            godot_warn!("SortTester: failed to compile sort_test.glsl – skipping tests.");
            return;
        };

        let mut passed = 0usize;
        let mut failed = 0usize;

        for (label, input) in TEST_CASES.iter() {
            match run_test_case(&mut rd, pipeline_rid, shader_rid, input) {
                Ok(output) => {
                    let mut expected = input.to_vec();
                    expected.sort_by(|a, b| a.partial_cmp(b).unwrap());

                    if output == expected {
                        godot_print!("[SortTester] PASS  '{}'", label);
                        passed += 1;
                    } else {
                        godot_print!(
                            "[SortTester] FAIL  '{}'\n  input:    {:?}\n  expected: {:?}\n  got:      {:?}",
                            label, input, expected, output
                        );
                        failed += 1;
                    }
                }
                Err(msg) => {
                    godot_print!("[SortTester] ERROR '{}': {}", label, msg);
                    failed += 1;
                }
            }
        }

        godot_print!(
            "[SortTester] Results: {}/{} passed.",
            passed,
            passed + failed
        );

        // Free GPU resources created specifically for this one-shot test.
        rd.free_rid(pipeline_rid);
        rd.free_rid(shader_rid);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compile the compute shader and return (shader_rid, pipeline_rid), or None on failure.
fn compile_pipeline(rd: &mut Gd<RenderingDevice>) -> Option<(Rid, Rid)> {
    let shader_file: Gd<RdShaderFile> =
        load("res://addons/beatsmr/shaders/sort_test.glsl");

    let spirv = shader_file.get_spirv()?;
    let shader_rid = rd.shader_create_from_spirv(&spirv);
    if shader_rid == Rid::Invalid {
        return None;
    }

    let pipeline_rid = rd.compute_pipeline_create(shader_rid);

    if pipeline_rid == Rid::Invalid {
        rd.free_rid(shader_rid);
        None
    } else {
        Some((shader_rid, pipeline_rid))
    }
}

/// Upload `keys`, dispatch the shader, read back the sorted keys.
fn run_test_case(
    rd: &mut Gd<RenderingDevice>,
    pipeline_rid: Rid,
    shader_rid: Rid,
    keys: &[f32],
) -> Result<Vec<f32>, String> {
    // Buffer layout: [n (as f32), key0, key1, ...]
    let n = keys.len() as u32;
    let element_count = 1 + keys.len(); // n header + keys
    let byte_count = (element_count * std::mem::size_of::<f32>()) as u32;

    let mut raw: Vec<u8> = Vec::with_capacity(byte_count as usize);
    raw.extend_from_slice(&(n as f32).to_le_bytes());
    for k in keys {
        raw.extend_from_slice(&k.to_le_bytes());
    }

    let packed = PackedByteArray::from(raw.as_slice());

    let buffer_rid = rd
        .storage_buffer_create_ex(byte_count)
        .data(&packed)
        .done();
    if buffer_rid == Rid::Invalid {
        return Err("failed to create storage buffer".into());
    }

    // Build uniform set.
    let uniform_set_rid = {
        let mut uniform = RdUniform::new_gd();
        uniform.set_uniform_type(UniformType::STORAGE_BUFFER);
        uniform.set_binding(0);
        uniform.add_id(buffer_rid);

        rd.uniform_set_create(&Array::from_iter([uniform]), shader_rid, 0)
    };
    if uniform_set_rid == Rid::Invalid {
        rd.free_rid(buffer_rid);
        return Err("failed to create uniform set".into());
    }

    // Dispatch (single work-group; the shader handles everything in one thread).
    let list = rd.compute_list_begin();
    rd.compute_list_bind_compute_pipeline(list, pipeline_rid);
    rd.compute_list_bind_uniform_set(list, uniform_set_rid, 0);
    rd.compute_list_dispatch(list, 1, 1, 1);
    rd.compute_list_end();

    // Flush to ensure the GPU has finished before reading back.
    rd.submit();
    rd.sync();

    // Read back.
    let result_bytes = rd.buffer_get_data(buffer_rid);
    let result_raw = result_bytes.to_vec();

    // Free per-test GPU objects.
    rd.free_rid(uniform_set_rid);
    rd.free_rid(buffer_rid);

    if result_raw.len() < byte_count as usize {
        return Err(format!(
            "short read: expected {} bytes, got {}",
            byte_count,
            result_raw.len()
        ));
    }

    // Skip the n-header, convert remaining bytes back to f32.
    let mut output = Vec::with_capacity(keys.len());
    for i in 0..keys.len() {
        let offset = (1 + i) * std::mem::size_of::<f32>();
        let bytes: [u8; 4] = result_raw[offset..offset + 4].try_into().unwrap();
        output.push(f32::from_le_bytes(bytes));
    }

    Ok(output)
}
