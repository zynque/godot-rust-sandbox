use godot::prelude::*;
use godot::classes::{
    Engine,
    RdUniform,
    RenderingDevice,
};
use godot::classes::rendering_device::UniformType;

use crate::shader_test_common::{compile_compute_pipeline, create_rendering_device};

// ---------------------------------------------------------------------------
// SortIntervalsTester
//
// A tool node that compiles the isolated sort_intervals() compute shader,
// uploads an unsorted list of parcel intervals, dispatches a single
// work-group, and verifies that the intervals come back ordered by entry.
// ---------------------------------------------------------------------------

const TEST_SHADER_PATH: &str =
    "res://addons/beatsmr/shaders/parcel_renderer/sort_intervals_test.glsl";
const CONTEXT: &str = "SortIntervalsTester";

/// Mirrors MAX_PARCEL_INTERVALS in constants.glslinc.
const MAX_PARCEL_INTERVALS: usize = 100;
/// Entry/exit come back from GPU float math, so they are compared with a
/// tolerance. Parcel indices are integers and must match exactly.
const EPSILON: f32 = 1e-4;

/// One (entry, exit, parcel_index) interval.
type Interval = (f32, f32, i32);

struct TestCase {
    label: &'static str,
    /// Intervals in the order they are uploaded to the shader.
    input: &'static [Interval],
    /// Intervals after sorting by entry, ties keeping their input order.
    expected: &'static [Interval],
}

const TEST_CASES: &[TestCase] = &[
    TestCase {
        label: "empty",
        input: &[],
        expected: &[],
    },
    TestCase {
        label: "single interval",
        input: &[(5.0, 10.0, 0)],
        expected: &[(5.0, 10.0, 0)],
    },
    TestCase {
        label: "already sorted",
        input: &[(1.0, 2.0, 0), (3.0, 4.0, 1), (5.0, 6.0, 2)],
        expected: &[(1.0, 2.0, 0), (3.0, 4.0, 1), (5.0, 6.0, 2)],
    },
    TestCase {
        label: "reverse order",
        input: &[(9.0, 10.0, 0), (5.0, 6.0, 1), (1.0, 2.0, 2)],
        expected: &[(1.0, 2.0, 2), (5.0, 6.0, 1), (9.0, 10.0, 0)],
    },
    TestCase {
        label: "interior insertion",
        input: &[(1.0, 2.0, 0), (9.0, 10.0, 1), (3.0, 4.0, 2)],
        expected: &[(1.0, 2.0, 0), (3.0, 4.0, 2), (9.0, 10.0, 1)],
    },
    TestCase {
        label: "ties keep input order",
        input: &[(3.0, 40.0, 0), (1.0, 20.0, 1), (3.0, 80.0, 2), (2.0, 90.0, 3)],
        expected: &[(1.0, 20.0, 1), (2.0, 90.0, 3), (3.0, 40.0, 0), (3.0, 80.0, 2)],
    },
    TestCase {
        label: "shuffled",
        input: &[
            (7.0, 8.0, 0),
            (2.0, 3.0, 1),
            (9.0, 10.0, 2),
            (1.0, 11.0, 3),
            (5.0, 6.0, 4),
        ],
        expected: &[
            (1.0, 11.0, 3),
            (2.0, 3.0, 1),
            (5.0, 6.0, 4),
            (7.0, 8.0, 0),
            (9.0, 10.0, 2),
        ],
    },
];

#[derive(GodotClass)]
#[class(tool, base=Node)]
pub struct SortIntervalsTester {
    base: Base<Node>,
}

#[godot_api]
impl INode for SortIntervalsTester {
    fn init(base: Base<Node>) -> Self {
        Self { base }
    }

    fn ready(&mut self) {}
}

#[godot_api]
impl SortIntervalsTester {
    #[func]
    fn run_tests(&mut self) {
        if !Engine::singleton().is_editor_hint() {
            return;
        }

        let Some(mut rd) = create_rendering_device(CONTEXT) else {
            return;
        };

        let Some((shader_rid, pipeline_rid)) =
            compile_compute_pipeline(&mut rd, TEST_SHADER_PATH, CONTEXT)
        else {
            return;
        };

        let mut passed = 0usize;
        let mut failed = 0usize;

        for case in TEST_CASES.iter() {
            match run_test_case(&mut rd, pipeline_rid, shader_rid, case) {
                Ok(intervals) => {
                    if intervals_match(&intervals, case.expected) {
                        godot_print!("[{}] PASS  '{}'", CONTEXT, case.label);
                        passed += 1;
                    } else {
                        godot_print!(
                            "[{}] FAIL  '{}'\n  expected: {:?}\n  got:      {:?}",
                            CONTEXT,
                            case.label,
                            case.expected,
                            intervals
                        );
                        failed += 1;
                    }
                }
                Err(msg) => {
                    godot_print!("[{}] ERROR '{}': {}", CONTEXT, case.label, msg);
                    failed += 1;
                }
            }
        }

        godot_print!(
            "[{}] Results: {}/{} passed.",
            CONTEXT,
            passed,
            passed + failed
        );

        rd.free_rid(pipeline_rid);
        rd.free_rid(shader_rid);
    }
}

fn intervals_match(actual: &[Interval], expected: &[Interval]) -> bool {
    actual.len() == expected.len()
        && actual.iter().zip(expected).all(|(a, e)| {
            (a.0 - e.0).abs() <= EPSILON && (a.1 - e.1).abs() <= EPSILON && a.2 == e.2
        })
}

/// Rejects test data the shader cannot represent, so a bad case fails loudly
/// instead of reading outside the interval array.
fn validate(case: &TestCase) -> Result<(), String> {
    if case.input.len() > MAX_PARCEL_INTERVALS {
        return Err(format!(
            "{} intervals exceeds MAX_PARCEL_INTERVALS ({})",
            case.input.len(),
            MAX_PARCEL_INTERVALS
        ));
    }
    Ok(())
}

/// Packs a case into the flat float layout the test shader expects.
fn encode_input(case: &TestCase) -> Vec<f32> {
    let mut data = Vec::with_capacity(1 + 3 * case.input.len());
    data.push(case.input.len() as f32);
    for (entry, exit, parcel_index) in case.input {
        data.extend_from_slice(&[*entry, *exit, *parcel_index as f32]);
    }
    data
}

fn run_test_case(
    rd: &mut Gd<RenderingDevice>,
    pipeline_rid: Rid,
    shader_rid: Rid,
    case: &TestCase,
) -> Result<Vec<Interval>, String> {
    validate(case)?;

    let input = encode_input(case);
    let mut input_bytes = Vec::with_capacity(input.len() * std::mem::size_of::<f32>());
    for value in &input {
        input_bytes.extend_from_slice(&value.to_le_bytes());
    }
    let input_byte_count = input_bytes.len() as u32;
    let output_float_count = 1 + 3 * MAX_PARCEL_INTERVALS;
    let output_byte_count = (output_float_count * std::mem::size_of::<f32>()) as u32;

    let input_buffer_rid = rd
        .storage_buffer_create_ex(input_byte_count)
        .data(&PackedByteArray::from(input_bytes.as_slice()))
        .done();
    if input_buffer_rid == Rid::Invalid {
        return Err("failed to create input storage buffer".into());
    }

    let zeroed = PackedByteArray::from(vec![0u8; output_byte_count as usize].as_slice());
    let output_buffer_rid = rd
        .storage_buffer_create_ex(output_byte_count)
        .data(&zeroed)
        .done();
    if output_buffer_rid == Rid::Invalid {
        rd.free_rid(input_buffer_rid);
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

        rd.uniform_set_create(
            &Array::from_iter([input_uniform, output_uniform]),
            shader_rid,
            0,
        )
    };
    if uniform_set_rid == Rid::Invalid {
        rd.free_rid(input_buffer_rid);
        rd.free_rid(output_buffer_rid);
        return Err("failed to create uniform set".into());
    }

    let list = rd.compute_list_begin();
    rd.compute_list_bind_compute_pipeline(list, pipeline_rid);
    rd.compute_list_bind_uniform_set(list, uniform_set_rid, 0);
    rd.compute_list_dispatch(list, 1, 1, 1);
    rd.compute_list_end();

    rd.submit();
    rd.sync();

    let result = rd.buffer_get_data(output_buffer_rid).to_vec();

    rd.free_rid(uniform_set_rid);
    rd.free_rid(input_buffer_rid);
    rd.free_rid(output_buffer_rid);

    if result.len() < output_byte_count as usize {
        return Err(format!(
            "short read: expected {} bytes, got {}",
            output_byte_count,
            result.len()
        ));
    }

    let mut values = Vec::with_capacity(output_float_count);
    for i in 0..output_float_count {
        let offset = i * std::mem::size_of::<f32>();
        let bytes: [u8; 4] = result[offset..offset + 4].try_into().unwrap();
        values.push(f32::from_le_bytes(bytes));
    }

    let count = values[0] as usize;
    if count > MAX_PARCEL_INTERVALS {
        return Err(format!(
            "shader reported {} intervals, expected at most {}",
            count, MAX_PARCEL_INTERVALS
        ));
    }

    let mut intervals = Vec::with_capacity(count);
    for i in 0..count {
        let base = 1 + 3 * i;
        intervals.push((values[base], values[base + 1], values[base + 2] as i32));
    }

    Ok(intervals)
}
