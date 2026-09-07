use godot::prelude::*;
use godot::classes::{
    Engine,
    RdUniform,
    RenderingDevice,
};
use godot::classes::rendering_device::UniformType;

use crate::shader_test_common::{compile_compute_pipeline, create_rendering_device};

// ---------------------------------------------------------------------------
// InsertionSortByFloatTester
//
// A tool node that compiles the isolated insertion-sort compute shader,
// uploads a key array plus source indices, dispatches a single work-group,
// and verifies that both the sorted keys and the sorted index mapping are
// correct.
// ---------------------------------------------------------------------------

const TEST_CASES: &[(&str, &[f32])] = &[
    ("empty", &[]),
    ("single", &[42.0]),
    ("already sorted", &[1.0, 2.0, 3.0, 4.0, 5.0]),
    ("reverse order", &[9.0, 7.0, 5.0, 3.0, 1.0]),
    ("mixed", &[3.5, 0.25, 2.0, 2.0, -1.0, 9.75]),
];

#[derive(GodotClass)]
#[class(tool, base=Node)]
pub struct InsertionSortByFloatTester {
    base: Base<Node>,
}

#[godot_api]
impl INode for InsertionSortByFloatTester {
    fn init(base: Base<Node>) -> Self {
        Self { base }
    }

    fn ready(&mut self) {}
}

#[godot_api]
impl InsertionSortByFloatTester {
    #[func]
    fn run_tests(&mut self) {
        if !Engine::singleton().is_editor_hint() {
            return;
        }

        let Some(mut rd) = create_rendering_device("InsertionSortByFloatTester") else {
            return;
        };

        let Some((shader_rid, pipeline_rid)) = compile_compute_pipeline(
            &mut rd,
            "res://addons/beatsmr/shaders/parcel_renderer/insertion_sort_by_float_test.glsl",
            "InsertionSortByFloatTester",
        ) else {
            return;
        };

        let mut passed = 0usize;
        let mut failed = 0usize;

        for (label, input) in TEST_CASES.iter() {
            match run_test_case(&mut rd, pipeline_rid, shader_rid, input) {
                Ok((sorted_keys, sorted_indices)) => {
                    let mut expected_keys = input.to_vec();
                    expected_keys.sort_by(|a, b| a.partial_cmp(b).unwrap());

                    let expected_indices = {
                        let mut pairs: Vec<(f32, u32)> = input
                            .iter()
                            .copied()
                            .enumerate()
                            .map(|(idx, value)| (value, idx as u32))
                            .collect();
                        pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                        pairs.into_iter().map(|(_, idx)| idx).collect::<Vec<_>>()
                    };

                    if sorted_keys == expected_keys && sorted_indices == expected_indices {
                        godot_print!("[InsertionSortByFloatTester] PASS  '{}'", label);
                        passed += 1;
                    } else {
                        godot_print!(
                            "[InsertionSortByFloatTester] FAIL  '{}'\n  input:    {:?}\n  expected_keys: {:?}\n  got_keys:      {:?}\n  expected_indices: {:?}\n  got_indices:      {:?}",
                            label,
                            input,
                            expected_keys,
                            sorted_keys,
                            expected_indices,
                            sorted_indices
                        );
                        failed += 1;
                    }
                }
                Err(msg) => {
                    godot_print!("[InsertionSortByFloatTester] ERROR '{}': {}", label, msg);
                    failed += 1;
                }
            }
        }

        godot_print!(
            "[InsertionSortByFloatTester] Results: {}/{} passed.",
            passed,
            passed + failed
        );

        rd.free_rid(pipeline_rid);
        rd.free_rid(shader_rid);
    }
}

fn run_test_case(
    rd: &mut Gd<RenderingDevice>,
    pipeline_rid: Rid,
    shader_rid: Rid,
    input: &[f32],
) -> Result<(Vec<f32>, Vec<u32>), String> {
    let size = input.len() as u32;
    let byte_count = (input.len().max(1) * std::mem::size_of::<f32>()) as u32;
    let index_byte_count = (input.len().max(1) * std::mem::size_of::<u32>()) as u32;

    let mut key_data = Vec::with_capacity(byte_count as usize);
    for value in input {
        key_data.extend_from_slice(&value.to_le_bytes());
    }
    if input.is_empty() {
        key_data.extend_from_slice(&(0.0f32).to_le_bytes());
    }

    let mut index_data = Vec::with_capacity(index_byte_count as usize);
    for i in 0..input.len() {
        index_data.extend_from_slice(&(i as u32).to_le_bytes());
    }
    if input.is_empty() {
        index_data.extend_from_slice(&(0u32).to_le_bytes());
    }

    let key_buffer_rid = rd
        .storage_buffer_create_ex(byte_count)
        .data(&PackedByteArray::from(key_data.as_slice()))
        .done();
    if key_buffer_rid == Rid::Invalid {
        return Err("failed to create key storage buffer".into());
    }

    let index_buffer_rid = rd
        .storage_buffer_create_ex(index_byte_count)
        .data(&PackedByteArray::from(index_data.as_slice()))
        .done();
    if index_buffer_rid == Rid::Invalid {
        rd.free_rid(key_buffer_rid);
        return Err("failed to create index storage buffer".into());
    }

    let size_buffer_rid = rd
        .storage_buffer_create_ex(std::mem::size_of::<u32>() as u32)
        .data(&PackedByteArray::from(size.to_le_bytes().as_slice()))
        .done();
    if size_buffer_rid == Rid::Invalid {
        rd.free_rid(key_buffer_rid);
        rd.free_rid(index_buffer_rid);
        return Err("failed to create size storage buffer".into());
    }

    let uniform_set_rid = {
        let mut key_uniform = RdUniform::new_gd();
        key_uniform.set_uniform_type(UniformType::STORAGE_BUFFER);
        key_uniform.set_binding(0);
        key_uniform.add_id(key_buffer_rid);

        let mut index_uniform = RdUniform::new_gd();
        index_uniform.set_uniform_type(UniformType::STORAGE_BUFFER);
        index_uniform.set_binding(1);
        index_uniform.add_id(index_buffer_rid);

        let mut size_uniform = RdUniform::new_gd();
        size_uniform.set_uniform_type(UniformType::STORAGE_BUFFER);
        size_uniform.set_binding(2);
        size_uniform.add_id(size_buffer_rid);

        rd.uniform_set_create(
            &Array::from_iter([key_uniform, index_uniform, size_uniform]),
            shader_rid,
            0,
        )
    };
    if uniform_set_rid == Rid::Invalid {
        rd.free_rid(key_buffer_rid);
        rd.free_rid(index_buffer_rid);
        rd.free_rid(size_buffer_rid);
        return Err("failed to create uniform set".into());
    }

    let list = rd.compute_list_begin();
    rd.compute_list_bind_compute_pipeline(list, pipeline_rid);
    rd.compute_list_bind_uniform_set(list, uniform_set_rid, 0);
    rd.compute_list_dispatch(list, 1, 1, 1);
    rd.compute_list_end();

    rd.submit();
    rd.sync();

    let key_result = rd.buffer_get_data(key_buffer_rid).to_vec();
    let index_result = rd.buffer_get_data(index_buffer_rid).to_vec();

    rd.free_rid(uniform_set_rid);
    rd.free_rid(key_buffer_rid);
    rd.free_rid(index_buffer_rid);
    rd.free_rid(size_buffer_rid);

    let mut sorted_keys = Vec::with_capacity(input.len());
    for i in 0..input.len() {
        let offset = i * std::mem::size_of::<f32>();
        let bytes: [u8; 4] = key_result[offset..offset + 4].try_into().unwrap();
        sorted_keys.push(f32::from_le_bytes(bytes));
    }

    let mut sorted_indices = Vec::with_capacity(input.len());
    for i in 0..input.len() {
        let offset = i * std::mem::size_of::<u32>();
        let bytes: [u8; 4] = index_result[offset..offset + 4].try_into().unwrap();
        sorted_indices.push(u32::from_le_bytes(bytes));
    }

    Ok((sorted_keys, sorted_indices))
}
