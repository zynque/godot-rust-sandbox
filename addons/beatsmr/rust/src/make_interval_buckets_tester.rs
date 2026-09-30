use godot::prelude::*;

use crate::parcel_test_common::{
    BucketWeights, INTERVAL_BUCKET_COUNT, INTERVAL_BUCKET_POINTS, Interval, MAX_PARCEL_INTERVALS,
    ParcelCase, bucket_weights_output_floats, decode_bucket_weights, parcel_test_node,
    verify_bucket_weights,
};

// ---------------------------------------------------------------------------
// MakeIntervalBucketsTester
//
// Compiles the isolated make_interval_buckets() compute shader, uploads a list
// of overlapping intervals, dispatches a single work-group, and verifies that
// the buckets span the cluster evenly and start with empty estimates centred on
// each bucket.
// ---------------------------------------------------------------------------

const TEST_SHADER_PATH: &str =
    "res://addons/beatsmr/shaders/parcel_renderer/make_interval_buckets_test.glsl";
const CONTEXT: &str = "MakeIntervalBucketsTester";

struct TestCase {
    label: &'static str,
    /// Intervals in the order they are uploaded to the shader.
    input: &'static [Interval],
}

const TEST_CASES: &[TestCase] = &[
    TestCase {
        label: "empty",
        input: &[],
    },
    TestCase {
        label: "single interval",
        input: &[(2.0, 10.0, 3)],
    },
    TestCase {
        label: "overlapping intervals",
        input: &[(1.0, 6.0, 0), (3.0, 9.0, 1)],
    },
    TestCase {
        label: "extent spans the wider interval",
        input: &[(5.0, 10.0, 1), (1.0, 4.0, 0)],
    },
    TestCase {
        label: "unsorted input keeps its order",
        input: &[(9.0, 12.0, 4), (1.0, 20.0, 2)],
    },
];

/// The buckets `make_interval_buckets()` should produce for an interval list.
fn expected_buckets(intervals: &[Interval]) -> BucketWeights {
    let parcel_indices: Vec<u32> = intervals
        .iter()
        .map(|(_, _, parcel_index)| *parcel_index as u32)
        .collect();

    let mut buckets = vec![0.0f32; INTERVAL_BUCKET_POINTS];
    let mut positions = vec![0.0f32; INTERVAL_BUCKET_COUNT];

    if let Some((first_entry, first_exit, _)) = intervals.first() {
        let mut front = *first_entry;
        let mut back = *first_exit;
        for (entry, exit, _) in intervals {
            front = front.min(*entry);
            back = back.max(*exit);
        }

        let span = back - front;
        for (i, bucket) in buckets.iter_mut().enumerate() {
            *bucket = front + span * i as f32 / INTERVAL_BUCKET_COUNT as f32;
        }
        for (i, position) in positions.iter_mut().enumerate() {
            *position = 0.5 * (buckets[i] + buckets[i + 1]);
        }
    }

    BucketWeights {
        parcel_count: intervals.len() as u32,
        buckets,
        positions,
        evidence: vec![0.0; INTERVAL_BUCKET_COUNT],
        deviations: vec![0.0; INTERVAL_BUCKET_COUNT],
        parcel_indices,
    }
}

impl ParcelCase for TestCase {
    fn label(&self) -> &'static str {
        self.label
    }

    fn encode_input(&self) -> Result<Vec<f32>, String> {
        if self.input.len() > MAX_PARCEL_INTERVALS {
            return Err(format!(
                "{} intervals exceeds MAX_PARCEL_INTERVALS ({})",
                self.input.len(),
                MAX_PARCEL_INTERVALS
            ));
        }

        let mut data = Vec::with_capacity(1 + 3 * self.input.len());
        data.push(self.input.len() as f32);
        for (entry, exit, parcel_index) in self.input {
            data.extend_from_slice(&[*entry, *exit, *parcel_index as f32]);
        }
        Ok(data)
    }

    fn output_floats(&self) -> usize {
        bucket_weights_output_floats()
    }

    fn verify(&self, output: &[f32]) -> Result<(), String> {
        verify_bucket_weights(
            &decode_bucket_weights(output)?,
            &expected_buckets(self.input),
        )
    }
}

parcel_test_node!(
    MakeIntervalBucketsTester,
    CONTEXT,
    TEST_SHADER_PATH,
    TEST_CASES
);
