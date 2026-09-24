use godot::prelude::*;

use crate::parcel_test_common::{
    Interval, IntervalCluster, MAX_PARCEL_INTERVALS, ParcelCase, decode_interval_cluster,
    interval_cluster_output_floats, parcel_test_node, verify_interval_cluster,
};

// ---------------------------------------------------------------------------
// BuildIntervalClusterTester
//
// Compiles the isolated build_interval_cluster() compute shader, uploads an
// interval list, dispatches a single work-group, and verifies the cluster's
// sizes, inflection points and parcel indices.
// ---------------------------------------------------------------------------

const TEST_SHADER_PATH: &str =
    "res://addons/beatsmr/shaders/parcel_renderer/build_interval_cluster_test.glsl";
const CONTEXT: &str = "BuildIntervalClusterTester";

struct TestCase {
    label: &'static str,
    /// Intervals in the order they are uploaded to the shader.
    input: &'static [Interval],
}

// build_interval_cluster() re-lays the intervals out without reordering them:
// each interval contributes its entry then its exit to the inflection points,
// and its parcel index to the parcel set. Sorting the inflection points is
// sort_interval_cluster()'s job, not this function's.
const TEST_CASES: &[TestCase] = &[
    TestCase {
        label: "empty",
        input: &[],
    },
    TestCase {
        label: "single interval",
        input: &[(5.0, 10.0, 7)],
    },
    TestCase {
        label: "two intervals",
        input: &[(1.0, 2.0, 0), (3.0, 4.0, 1)],
    },
    TestCase {
        label: "unsorted input keeps its order",
        input: &[(9.0, 10.0, 2), (1.0, 2.0, 0)],
    },
    TestCase {
        label: "parcel indices are preserved",
        input: &[(1.0, 2.0, 5), (3.0, 4.0, 9)],
    },
];

/// The cluster `build_interval_cluster()` should produce for an interval list.
fn expected_cluster(intervals: &[Interval]) -> IntervalCluster {
    let mut inflection_points = Vec::with_capacity(2 * intervals.len());
    let mut parcel_indices = Vec::with_capacity(intervals.len());
    for (entry, exit, parcel_index) in intervals {
        inflection_points.push(*entry);
        inflection_points.push(*exit);
        parcel_indices.push(*parcel_index as u32);
    }

    IntervalCluster {
        size: (2 * intervals.len()) as u32,
        parcel_count: intervals.len() as u32,
        inflection_points,
        parcel_indices,
    }
}

impl ParcelCase for TestCase {
    fn label(&self) -> &'static str {
        self.label
    }

    fn encode_input(&self) -> Result<Vec<f32>, String> {
        // Every interval contributes two inflection points to a
        // float[MAX_PARCELS] array, so only half the capacity is usable.
        if 2 * self.input.len() > MAX_PARCEL_INTERVALS {
            return Err(format!(
                "{} intervals need {} inflection points, exceeding MAX_PARCELS ({})",
                self.input.len(),
                2 * self.input.len(),
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
        interval_cluster_output_floats()
    }

    fn verify(&self, output: &[f32]) -> Result<(), String> {
        verify_interval_cluster(
            &decode_interval_cluster(output)?,
            &expected_cluster(self.input),
        )
    }
}

parcel_test_node!(
    BuildIntervalClusterTester,
    CONTEXT,
    TEST_SHADER_PATH,
    TEST_CASES
);
