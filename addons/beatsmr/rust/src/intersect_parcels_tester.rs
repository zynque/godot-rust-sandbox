use godot::prelude::*;
use godot::classes::{
    Engine,
    RdUniform,
    RenderingDevice,
};
use godot::classes::rendering_device::UniformType;

use crate::shader_test_common::{compile_compute_pipeline, create_rendering_device};

// ---------------------------------------------------------------------------
// IntersectParcelsTester
//
// A tool node that compiles the isolated intersect_parcels() compute shader,
// uploads a ray, a parcel buffer and a set of parcel indices, dispatches a
// single work-group, and verifies the reported ray/parcel boundary intervals.
// ---------------------------------------------------------------------------

const TEST_SHADER_PATH: &str =
    "res://addons/beatsmr/shaders/parcel_renderer/intersect_parcels_test.glsl";
const CONTEXT: &str = "IntersectParcelsTester";

/// Mirrors MAX_PARCELS in constants.glslinc.
const MAX_PARCELS: usize = 100;
/// Mirrors MAX_PARCEL_INTERVALS in constants.glslinc.
const MAX_PARCEL_INTERVALS: usize = 100;
/// Entry/exit come back from GPU float math, so they are compared with a
/// tolerance. Parcel indices are integers and must match exactly.
const EPSILON: f32 = 1e-4;

/// The subset of `Parcel` that `intersect_parcels` reads. `inverse_variance`
/// is the diagonal of the inverse covariance; all test parcels are
/// axis aligned, so the off-diagonal entries are zero. A sigma of 1.0 means
/// an inverse variance of 1.0, a sigma of 0.5 means 4.0, and so on.
#[derive(Clone, Copy)]
struct TestParcel {
    mean: [f32; 3],
    inverse_variance: [f32; 3],
}

/// An axis aligned parcel centred at `center` with the given inverse variance
/// diagonal, in the order x, y, z.
const fn parcel(center: [f32; 3], inverse_variance: [f32; 3]) -> TestParcel {
    TestParcel {
        mean: center,
        inverse_variance,
    }
}

const UNIT_SPHERE_AT_Z_4: TestParcel = parcel([0.0, 0.0, 4.0], [1.0, 1.0, 1.0]);
const UNIT_SPHERE_AT_Z_7: TestParcel = parcel([0.0, 0.0, 7.0], [1.0, 1.0, 1.0]);
const UNIT_SPHERE_AT_Z_10: TestParcel = parcel([0.0, 0.0, 10.0], [1.0, 1.0, 1.0]);
const UNIT_SPHERE_AT_Z_20: TestParcel = parcel([0.0, 0.0, 20.0], [1.0, 1.0, 1.0]);
/// Sigmas of (1.0, 2.0, 0.5) along x, y, z.
const STRETCHED_PARCEL_AT_Z_10: TestParcel = parcel([0.0, 0.0, 10.0], [1.0, 0.25, 4.0]);

struct TestCase {
    label: &'static str,
    ray_origin: [f32; 3],
    ray_direction: [f32; 3],
    parcels: &'static [TestParcel],
    set_indices: &'static [u32],
    /// (entry, exit, parcel_index) per interval, in set order.
    expected: &'static [(f32, f32, i32)],
}

// A ray of entry/exit t is a parcel's 3 sigma boundary, so a unit sigma parcel
// centred at z = 10 is hit between t = 7 and t = 13.
const TEST_CASES: &[TestCase] = &[
    TestCase {
        label: "empty set",
        ray_origin: [0.0, 0.0, 0.0],
        ray_direction: [0.0, 0.0, 1.0],
        parcels: &[UNIT_SPHERE_AT_Z_10],
        set_indices: &[],
        expected: &[],
    },
    TestCase {
        label: "single parcel hit",
        ray_origin: [0.0, 0.0, 0.0],
        ray_direction: [0.0, 0.0, 1.0],
        parcels: &[UNIT_SPHERE_AT_Z_10],
        set_indices: &[0],
        expected: &[(7.0, 13.0, 0)],
    },
    TestCase {
        label: "ray misses parcel",
        ray_origin: [0.0, 0.0, 0.0],
        ray_direction: [1.0, 0.0, 0.0],
        parcels: &[UNIT_SPHERE_AT_Z_10],
        set_indices: &[0],
        expected: &[],
    },
    TestCase {
        label: "anisotropic parcel",
        ray_origin: [0.0, 0.0, 0.0],
        ray_direction: [0.0, 0.0, 1.0],
        parcels: &[STRETCHED_PARCEL_AT_Z_10],
        set_indices: &[0],
        expected: &[(8.5, 11.5, 0)],
    },
    TestCase {
        label: "tangent ray",
        ray_origin: [3.0, 0.0, 0.0],
        ray_direction: [0.0, 0.0, 1.0],
        parcels: &[UNIT_SPHERE_AT_Z_10],
        set_indices: &[0],
        expected: &[(10.0, 10.0, 0)],
    },
    TestCase {
        label: "set order preserved",
        ray_origin: [0.0, 0.0, 0.0],
        ray_direction: [0.0, 0.0, 1.0],
        parcels: &[UNIT_SPHERE_AT_Z_10, UNIT_SPHERE_AT_Z_4, UNIT_SPHERE_AT_Z_7],
        set_indices: &[0, 1, 2],
        expected: &[(7.0, 13.0, 0), (1.0, 7.0, 1), (4.0, 10.0, 2)],
    },
    TestCase {
        label: "parcel_index is the buffer index",
        ray_origin: [0.0, 0.0, 0.0],
        ray_direction: [0.0, 0.0, 1.0],
        parcels: &[UNIT_SPHERE_AT_Z_10, UNIT_SPHERE_AT_Z_4, UNIT_SPHERE_AT_Z_20],
        set_indices: &[2, 0],
        expected: &[(17.0, 23.0, 2), (7.0, 13.0, 0)],
    },
];

#[derive(GodotClass)]
#[class(tool, base=Node)]
pub struct IntersectParcelsTester {
    base: Base<Node>,
}

#[godot_api]
impl INode for IntersectParcelsTester {
    fn init(base: Base<Node>) -> Self {
        Self { base }
    }

    fn ready(&mut self) {}
}

#[godot_api]
impl IntersectParcelsTester {
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

fn intervals_match(actual: &[(f32, f32, i32)], expected: &[(f32, f32, i32)]) -> bool {
    actual.len() == expected.len()
        && actual.iter().zip(expected).all(|(a, e)| {
            (a.0 - e.0).abs() <= EPSILON && (a.1 - e.1).abs() <= EPSILON && a.2 == e.2
        })
}

/// Rejects test data the shader cannot represent, so a bad case fails loudly
/// instead of reading outside the parcel buffer.
fn validate(case: &TestCase) -> Result<(), String> {
    if case.parcels.len() > MAX_PARCELS {
        return Err(format!(
            "{} parcels exceeds MAX_PARCELS ({})",
            case.parcels.len(),
            MAX_PARCELS
        ));
    }
    if case.set_indices.len() > MAX_PARCELS {
        return Err(format!(
            "set of {} indices exceeds MAX_PARCELS ({})",
            case.set_indices.len(),
            MAX_PARCELS
        ));
    }
    for index in case.set_indices {
        if *index as usize >= case.parcels.len() {
            return Err(format!(
                "set index {} is out of range for {} parcels",
                index,
                case.parcels.len()
            ));
        }
    }
    Ok(())
}

/// Packs a case into the flat float layout the test shader expects.
fn encode_input(case: &TestCase) -> Vec<f32> {
    let mut data = Vec::new();
    data.extend_from_slice(&case.ray_origin);
    data.extend_from_slice(&case.ray_direction);

    data.push(case.parcels.len() as f32);
    for parcel in case.parcels {
        let [x, y, z] = parcel.inverse_variance;
        data.extend_from_slice(&parcel.mean);
        // Column major diagonal.
        data.extend_from_slice(&[x, 0.0, 0.0, 0.0, y, 0.0, 0.0, 0.0, z]);
    }

    data.push(case.set_indices.len() as f32);
    data.extend(case.set_indices.iter().map(|index| *index as f32));

    data
}

fn run_test_case(
    rd: &mut Gd<RenderingDevice>,
    pipeline_rid: Rid,
    shader_rid: Rid,
    case: &TestCase,
) -> Result<Vec<(f32, f32, i32)>, String> {
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
