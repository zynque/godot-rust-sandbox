use godot::prelude::*;

use crate::parcel_test_common::{
    BucketWeights, INTERVAL_BUCKET_COUNT, INTERVAL_BUCKET_POINTS, ParcelCase,
    bucket_weights_output_floats, decode_bucket_weights, parcel_test_node, verify_bucket_weights,
};

// ---------------------------------------------------------------------------
// NarrowIntervalBucketWeightsTester
//
// Compiles the isolated narrow_interval_bucket_weights() compute shader,
// uploads a ready made set of interval buckets and a density threshold,
// dispatches a single work-group, and verifies that the shader drops the
// sub-threshold buckets at either end, restratifies the kept span evenly and
// pools the old estimates (merging those that land together, and giving any
// bucket left empty the unobserved-bucket prior).
// ---------------------------------------------------------------------------

const TEST_SHADER_PATH: &str =
    "res://addons/beatsmr/shaders/parcel_renderer/tests/narrow_interval_bucket_weights_test.glsl";
const CONTEXT: &str = "NarrowIntervalBucketWeightsTester";

/// Mirrors INTERVAL_BUCKET_UNOBSERVED_EVIDENCE_FRACTION in constants.glslinc.
const UNOBSERVED_EVIDENCE_FRACTION: f32 = 0.5;

/// The threshold the renderer uses: a fraction of ISOSURFACE.
const THRESHOLD: f32 = 0.25;

/// Evenly stratified bucket boundaries over the full [0, 16] extent, the shape
/// make_interval_buckets() gives a cluster that spans the whole range.
const FULL_BOUNDARIES: [f32; INTERVAL_BUCKET_POINTS] = [
    0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
];

/// The bucket midpoints that match FULL_BOUNDARIES.
const FULL_POSITIONS: [f32; INTERVAL_BUCKET_COUNT] = [
    0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5, 9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5,
];

struct TestCase {
    label: &'static str,
    buckets: [f32; INTERVAL_BUCKET_POINTS],
    positions: [f32; INTERVAL_BUCKET_COUNT],
    evidence: [f32; INTERVAL_BUCKET_COUNT],
    samples: [u32; INTERVAL_BUCKET_COUNT],
    deviations: [f32; INTERVAL_BUCKET_COUNT],
    threshold: f32,
}

/// Squared density every bucket observed, except the closed span [first, last]
/// which observed the higher squared density.
const fn evidence_between(
    low_density: f32,
    first: usize,
    last: usize,
    high_density: f32,
) -> [f32; INTERVAL_BUCKET_COUNT] {
    let mut evidence = [low_density * low_density; INTERVAL_BUCKET_COUNT];
    let mut i = first;
    while i <= last {
        evidence[i] = high_density * high_density;
        i += 1;
    }
    evidence
}

/// Every bucket sampled once with the given squared density, sitting at its
/// midpoint with no spread yet.
const fn uniform_case(
    label: &'static str,
    evidence: [f32; INTERVAL_BUCKET_COUNT],
    threshold: f32,
) -> TestCase {
    TestCase {
        label,
        buckets: FULL_BOUNDARIES,
        positions: FULL_POSITIONS,
        evidence,
        samples: [1u32; INTERVAL_BUCKET_COUNT],
        deviations: [0.0; INTERVAL_BUCKET_COUNT],
        threshold,
    }
}

/// Two adjacent, multiply sampled buckets whose estimates have drifted together
/// so they land in the same new bucket and are pooled.
const fn merge_case() -> TestCase {
    let mut case = uniform_case(
        "drifted estimates that land together are merged",
        evidence_between(0.0, 3, 12, 0.6),
        THRESHOLD,
    );
    case.positions[5] = 5.9;
    case.evidence[5] = 0.25;
    case.samples[5] = 2;
    case.deviations[5] = 0.02;
    case.positions[6] = 6.05;
    case.evidence[6] = 0.64;
    case.samples[6] = 1;
    case.deviations[6] = 0.01;
    case
}

const TEST_CASES: &[TestCase] = &[
    uniform_case(
        "no bucket reaches the threshold keeps the range",
        [0.25; INTERVAL_BUCKET_COUNT],
        THRESHOLD,
    ),
    uniform_case(
        "a single bucket restratifies onto its own span",
        evidence_between(0.0, 5, 5, 1.0),
        THRESHOLD,
    ),
    uniform_case(
        "trims both ends and carries the estimates inward",
        evidence_between(0.0, 3, 12, 0.6),
        THRESHOLD,
    ),
    merge_case(),
];

/// Index of the bucket containing `t`, mirroring find_interval_bucket().
fn find_bucket(boundaries: &[f32], t: f32) -> Option<usize> {
    for i in 0..INTERVAL_BUCKET_COUNT {
        let is_last = i == INTERVAL_BUCKET_COUNT - 1;
        let inside =
            t >= boundaries[i] && (t < boundaries[i + 1] || (is_last && t <= boundaries[i + 1]));
        if inside {
            return Some(i);
        }
    }
    None
}

/// Mirrors merge_interval_bucket_estimate() in
/// cluster/interval_bucket_narrowing.glslinc.
fn merge_estimate(target: &mut BucketWeights, index: usize, source: &BucketWeights, s: usize) {
    let source_samples = source.samples[s];
    if source_samples == 0 {
        return;
    }

    let source_weight = source.evidence[s] * source_samples as f32;
    let target_weight = target.evidence[index] * target.samples[index] as f32;
    let combined_weight = target_weight + source_weight;
    if combined_weight <= 0.0 {
        return;
    }

    let source_position = source.positions[s];
    let target_position = target.positions[index];
    let combined_position =
        (target_weight * target_position + source_weight * source_position) / combined_weight;

    target.deviations[index] += target_weight
        * (target_position - combined_position)
        * (target_position - combined_position)
        + source.deviations[s]
        + source_weight * (source_position - combined_position)
            * (source_position - combined_position);
    target.positions[index] = combined_position;
    target.samples[index] += source_samples;
    target.evidence[index] = combined_weight / target.samples[index] as f32;
}

/// Mirrors apply_interval_bucket_prior() in cluster/interval_bucket_sampling.glslinc.
fn apply_prior(buckets: &mut BucketWeights, observed: &[bool]) {
    let observed_count = observed.iter().filter(|seen| **seen).count();
    if observed_count == 0 {
        return;
    }

    let observed_evidence: f32 = (0..INTERVAL_BUCKET_COUNT)
        .filter(|i| observed[*i])
        .map(|i| buckets.evidence[i])
        .sum();
    let prior = UNOBSERVED_EVIDENCE_FRACTION * observed_evidence / observed_count as f32;

    for i in 0..INTERVAL_BUCKET_COUNT {
        if observed[i] {
            continue;
        }
        buckets.positions[i] = 0.5 * (buckets.buckets[i] + buckets.buckets[i + 1]);
        buckets.evidence[i] = prior;
    }
}

/// The buckets narrow_interval_bucket_weights() should return for a case.
fn expected_narrow(case: &TestCase) -> BucketWeights {
    let input = BucketWeights {
        parcel_count: 0,
        buckets: case.buckets.to_vec(),
        positions: case.positions.to_vec(),
        evidence: case.evidence.to_vec(),
        samples: case.samples.to_vec(),
        deviations: case.deviations.to_vec(),
        parcel_indices: Vec::new(),
    };

    let mut first: Option<usize> = None;
    let mut last = 0usize;
    for i in 0..INTERVAL_BUCKET_COUNT {
        if case.evidence[i].sqrt() >= case.threshold {
            if first.is_none() {
                first = Some(i);
            }
            last = i;
        }
    }
    let Some(first) = first else {
        return input;
    };

    // One bucket of margin on each side so the span leads in below the threshold.
    let first = (first as i32 - 1).max(0) as usize;
    let last = (last + 1).min(INTERVAL_BUCKET_COUNT - 1);

    let mut boundaries = [0.0f32; INTERVAL_BUCKET_POINTS];
    let front = case.buckets[first];
    let span = case.buckets[last + 1] - front;
    for (i, boundary) in boundaries.iter_mut().enumerate() {
        *boundary = front + span * i as f32 / INTERVAL_BUCKET_COUNT as f32;
    }

    let mut narrowed = BucketWeights {
        parcel_count: 0,
        buckets: boundaries.to_vec(),
        positions: (0..INTERVAL_BUCKET_COUNT)
            .map(|i| 0.5 * (boundaries[i] + boundaries[i + 1]))
            .collect(),
        evidence: vec![0.0; INTERVAL_BUCKET_COUNT],
        samples: vec![0; INTERVAL_BUCKET_COUNT],
        deviations: vec![0.0; INTERVAL_BUCKET_COUNT],
        parcel_indices: Vec::new(),
    };

    let mut observed = [false; INTERVAL_BUCKET_COUNT];
    for i in 0..INTERVAL_BUCKET_COUNT {
        let Some(index) = find_bucket(&boundaries, case.positions[i]) else {
            continue;
        };
        merge_estimate(&mut narrowed, index, &input, i);
        if case.samples[i] > 0 {
            observed[index] = true;
        }
    }

    apply_prior(&mut narrowed, &observed);
    narrowed
}

impl ParcelCase for TestCase {
    fn label(&self) -> &'static str {
        self.label
    }

    fn encode_input(&self) -> Result<Vec<f32>, String> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.buckets);
        data.extend_from_slice(&self.positions);
        data.extend_from_slice(&self.evidence);
        data.extend(self.samples.iter().map(|s| *s as f32));
        data.extend_from_slice(&self.deviations);
        data.push(self.threshold);
        Ok(data)
    }

    fn output_floats(&self) -> usize {
        bucket_weights_output_floats()
    }

    fn verify(&self, output: &[f32]) -> Result<(), String> {
        verify_bucket_weights(&decode_bucket_weights(output)?, &expected_narrow(self))
    }
}

parcel_test_node!(
    NarrowIntervalBucketWeightsTester,
    CONTEXT,
    TEST_SHADER_PATH,
    TEST_CASES
);
