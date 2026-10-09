use godot::classes::Engine;
use godot::prelude::*;

use crate::shader_test_common::ComputeHarness;

// ---------------------------------------------------------------------------
// Shared plumbing for tests that run a compute shader over parcel data.
//
// Every case uploads a flat float payload and validates the flat float
// response itself, so shaders with different output shapes (an interval list,
// an interval cluster, ...) share one trait and one runner.
// ---------------------------------------------------------------------------

/// Mirrors MAX_PARCEL_INTERVALS in constants.glslinc.
pub const MAX_PARCEL_INTERVALS: usize = 100;
/// Values come back from GPU float math, so they are compared with a
/// tolerance. Counts and indices must match exactly.
pub const EPSILON: f32 = 1e-4;

/// One (entry, exit, parcel_index) interval.
pub type Interval = (f32, f32, i32);

/// The parts of a `ParcelIntervalCluster` the tests inspect.
#[derive(Debug, Clone)]
pub struct IntervalCluster {
    /// cluster.size: the number of inflection points (two per interval).
    pub size: u32,
    /// cluster.parcels.size: the number of parcel indices.
    pub parcel_count: u32,
    /// cluster.inflection_points[0 .. size), entry then exit per interval.
    pub inflection_points: Vec<f32>,
    /// cluster.parcels.indices[0 .. parcel_count).
    pub parcel_indices: Vec<u32>,
}

/// A case for a compute shader test: a flat float payload in, a flat float
/// response out, and the logic that decides whether the response is correct.
pub trait ParcelCase {
    fn label(&self) -> &'static str;
    /// Flat float payload for binding 0, or an error if the case cannot be
    /// represented by the shader.
    fn encode_input(&self) -> Result<Vec<f32>, String>;
    /// Floats the shader writes to binding 1.
    fn output_floats(&self) -> usize;
    /// Validates the shader response, describing the mismatch on failure.
    fn verify(&self, output: &[f32]) -> Result<(), String>;
}

/// How many cases a tester passed out of how many it attempted.
#[derive(Clone, Copy)]
pub struct TestReport {
    pub passed: usize,
    pub total: usize,
}

impl TestReport {
    /// Godot facing form: x is `passed`, y is `total`.
    pub fn to_vector2i(self) -> Vector2i {
        Vector2i::new(self.passed as i32, self.total as i32)
    }
}

/// Runs every case against `shader_path`, prints a PASS/FAIL/ERROR report and
/// returns the counts.
///
/// Does nothing outside the editor, where the tests are driven from a scene's
/// tool button.
pub fn run_cases<C: ParcelCase>(context: &str, shader_path: &str, cases: &[C]) -> TestReport {
    let total = cases.len();

    if !Engine::singleton().is_editor_hint() {
        return TestReport { passed: 0, total: 0 };
    }

    let Some(mut harness) = ComputeHarness::new(context, shader_path) else {
        godot_print!("[{}] ERROR could not run any of the {} cases.", context, total);
        return TestReport { passed: 0, total };
    };

    let mut passed = 0usize;
    let mut failed = 0usize;

    for case in cases {
        let label = case.label();
        match run_case(&mut harness, case) {
            Ok(()) => {
                godot_print!("[{}] PASS  '{}'", context, label);
                passed += 1;
            }
            Err(CaseError::Mismatch(msg)) => {
                godot_print!("[{}] FAIL  '{}'\n  {}", context, label, msg);
                failed += 1;
            }
            Err(CaseError::Broken(msg)) => {
                godot_print!("[{}] ERROR '{}': {}", context, label, msg);
                failed += 1;
            }
        }
    }

    godot_print!(
        "[{}] Results: {}/{} passed.",
        context,
        passed,
        passed + failed
    );

    TestReport {
        passed,
        total: passed + failed,
    }
}

/// Declares the tool node for a shader test: a `#[class]` Node whose
/// `run_tests` method drives `run_cases` and returns its counts.
macro_rules! parcel_test_node {
    ($name:ident, $context:expr, $shader_path:expr, $cases:expr) => {
        #[derive(GodotClass)]
        #[class(tool, base=Node)]
        pub struct $name {
            base: Base<Node>,
        }

        #[godot_api]
        impl INode for $name {
            fn init(base: Base<Node>) -> Self {
                Self { base }
            }
        }

        #[godot_api]
        impl $name {
            /// Runs the cases and returns (passed, total).
            #[func]
            fn run_tests(&mut self) -> godot::builtin::Vector2i {
                $crate::parcel_test_common::run_cases($context, $shader_path, $cases).to_vector2i()
            }
        }
    };
}
pub(crate) use parcel_test_node;

/// Why a case did not pass: the shader could not be run, or its response was
/// wrong.
enum CaseError {
    Broken(String),
    Mismatch(String),
}

fn run_case<C: ParcelCase>(harness: &mut ComputeHarness, case: &C) -> Result<(), CaseError> {
    let input = case.encode_input().map_err(CaseError::Broken)?;
    let output = harness
        .dispatch(&input, case.output_floats())
        .map_err(CaseError::Broken)?;
    case.verify(&output).map_err(CaseError::Mismatch)
}

/// Floats an interval shader writes to binding 1: a count, then entry, exit
/// and parcel index per interval.
pub fn interval_output_floats() -> usize {
    1 + 3 * MAX_PARCEL_INTERVALS
}

pub fn decode_intervals(values: &[f32]) -> Result<Vec<Interval>, String> {
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

pub fn verify_intervals(actual: &[Interval], expected: &[Interval]) -> Result<(), String> {
    let matches = actual.len() == expected.len()
        && actual.iter().zip(expected).all(|(a, e)| {
            (a.0 - e.0).abs() <= EPSILON && (a.1 - e.1).abs() <= EPSILON && a.2 == e.2
        });

    if matches {
        Ok(())
    } else {
        Err(format!("expected: {:?}\n  got:      {:?}", expected, actual))
    }
}

/// Floats an interval cluster shader writes to binding 1: the cluster size,
/// the parcel count, then the inflection points, then the parcel indices. Both
/// arrays are sized MAX_PARCELS in structs.glslinc.
pub fn interval_cluster_output_floats() -> usize {
    2 + 2 * MAX_PARCEL_INTERVALS
}

pub fn decode_interval_cluster(values: &[f32]) -> Result<IntervalCluster, String> {
    let size = values[0] as usize;
    let parcel_count = values[1] as usize;

    if size > MAX_PARCEL_INTERVALS {
        return Err(format!(
            "shader reported cluster size {}, expected at most {}",
            size, MAX_PARCEL_INTERVALS
        ));
    }
    if parcel_count > MAX_PARCEL_INTERVALS {
        return Err(format!(
            "shader reported {} parcels, expected at most {}",
            parcel_count, MAX_PARCEL_INTERVALS
        ));
    }

    let points_start = 2;
    let indices_start = points_start + size;

    Ok(IntervalCluster {
        size: size as u32,
        parcel_count: parcel_count as u32,
        inflection_points: values[points_start..indices_start].to_vec(),
        parcel_indices: values[indices_start..indices_start + parcel_count]
            .iter()
            .map(|value| *value as u32)
            .collect(),
    })
}

pub fn verify_interval_cluster(
    actual: &IntervalCluster,
    expected: &IntervalCluster,
) -> Result<(), String> {
    let matches = actual.size == expected.size
        && actual.parcel_count == expected.parcel_count
        && actual.parcel_indices == expected.parcel_indices
        && floats_match(&actual.inflection_points, &expected.inflection_points);

    if matches {
        Ok(())
    } else {
        Err(format!("expected: {:?}\n  got:      {:?}", expected, actual))
    }
}

/// Mirrors INTERVAL_BUCKET_COUNT in constants.glslinc.
pub const INTERVAL_BUCKET_COUNT: usize = 16;
/// Mirrors INTERVAL_BUCKET_POINTS: one boundary position per bucket plus one.
pub const INTERVAL_BUCKET_POINTS: usize = INTERVAL_BUCKET_COUNT + 1;

/// The parts of a `ParcelIntervalBucketWeights16` the tests inspect.
#[derive(Debug, Clone)]
pub struct BucketWeights {
    /// buckets.parcels.size: the number of parcel indices.
    pub parcel_count: u32,
    /// buckets.buckets[0 .. INTERVAL_BUCKET_POINTS), rising front to back.
    pub buckets: Vec<f32>,
    /// buckets.positions[0 .. INTERVAL_BUCKET_COUNT): estimated positions.
    pub positions: Vec<f32>,
    /// buckets.evidence[0 .. INTERVAL_BUCKET_COUNT): average squared density.
    pub evidence: Vec<f32>,
    /// buckets.samples[0 .. INTERVAL_BUCKET_COUNT): number of samples tested.
    pub samples: Vec<u32>,
    /// buckets.deviations[0 .. INTERVAL_BUCKET_COUNT): weighted squared
    /// deviations around the estimated positions.
    pub deviations: Vec<f32>,
    /// buckets.parcels.indices[0 .. parcel_count).
    pub parcel_indices: Vec<u32>,
}

/// Floats a bucket weights shader writes to binding 1: the parcel count, then
/// the bucket boundaries, then the positions, the evidence, the samples and the
/// deviations, then the parcel indices. The parcel set is sized MAX_PARCELS in
/// structs.glslinc.
pub fn bucket_weights_output_floats() -> usize {
    1 + INTERVAL_BUCKET_POINTS
        + 4 * INTERVAL_BUCKET_COUNT
        + MAX_PARCEL_INTERVALS
}

pub fn decode_bucket_weights(values: &[f32]) -> Result<BucketWeights, String> {
    let parcel_count = values[0] as usize;
    if parcel_count > MAX_PARCEL_INTERVALS {
        return Err(format!(
            "shader reported {} parcels, expected at most {}",
            parcel_count, MAX_PARCEL_INTERVALS
        ));
    }

    let buckets_start = 1;
    let positions_start = buckets_start + INTERVAL_BUCKET_POINTS;
    let evidence_start = positions_start + INTERVAL_BUCKET_COUNT;
    let samples_start = evidence_start + INTERVAL_BUCKET_COUNT;
    let deviations_start = samples_start + INTERVAL_BUCKET_COUNT;
    let indices_start = deviations_start + INTERVAL_BUCKET_COUNT;

    Ok(BucketWeights {
        parcel_count: parcel_count as u32,
        buckets: values[buckets_start..positions_start].to_vec(),
        positions: values[positions_start..evidence_start].to_vec(),
        evidence: values[evidence_start..samples_start].to_vec(),
        samples: values[samples_start..deviations_start]
            .iter()
            .map(|value| *value as u32)
            .collect(),
        deviations: values[deviations_start..indices_start].to_vec(),
        parcel_indices: values[indices_start..indices_start + parcel_count]
            .iter()
            .map(|value| *value as u32)
            .collect(),
    })
}

pub fn verify_bucket_weights(
    actual: &BucketWeights,
    expected: &BucketWeights,
) -> Result<(), String> {
    let matches = actual.parcel_count == expected.parcel_count
        && actual.parcel_indices == expected.parcel_indices
        && actual.samples == expected.samples
        && floats_match(&actual.buckets, &expected.buckets)
        && floats_match(&actual.positions, &expected.positions)
        && floats_match(&actual.evidence, &expected.evidence)
        && floats_match(&actual.deviations, &expected.deviations);

    if matches {
        Ok(())
    } else {
        Err(format!("expected: {:?}\n  got:      {:?}", expected, actual))
    }
}

fn floats_match(actual: &[f32], expected: &[f32]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(a, e)| (a - e).abs() <= EPSILON)
}
