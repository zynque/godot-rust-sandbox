use godot::prelude::*;

use crate::parcel_test_common::{
    IntervalCluster, MAX_PARCEL_INTERVALS, ParcelCase, decode_interval_cluster,
    interval_cluster_output_floats, parcel_test_node, verify_interval_cluster,
};

// ---------------------------------------------------------------------------
// SortIntervalClusterTester
//
// Compiles the isolated sort_interval_cluster() compute shader, uploads a
// cluster, dispatches a single work-group, and verifies that the inflection
// points come back ascending without disturbing the parcel set.
// ---------------------------------------------------------------------------

const TEST_SHADER_PATH: &str =
    "res://addons/beatsmr/shaders/parcel_renderer/sort_interval_cluster_test.glsl";
const CONTEXT: &str = "SortIntervalClusterTester";

struct TestCase {
    label: &'static str,
    /// cluster.inflection_points, in upload order.
    points: &'static [f32],
    /// cluster.parcels.indices, in upload order.
    parcel_indices: &'static [u32],
}

// sort_interval_cluster() sorts cluster.inflection_points in place and leaves
// the parcel set alone.
const TEST_CASES: &[TestCase] = &[
    TestCase {
        label: "empty",
        points: &[],
        parcel_indices: &[],
    },
    TestCase {
        label: "single point",
        points: &[5.0],
        parcel_indices: &[3],
    },
    TestCase {
        label: "already sorted",
        points: &[1.0, 2.0, 3.0, 4.0],
        parcel_indices: &[],
    },
    TestCase {
        label: "reverse order",
        points: &[9.0, 7.0, 5.0, 3.0, 1.0],
        parcel_indices: &[],
    },
    TestCase {
        label: "interleaved entries and exits",
        points: &[7.0, 13.0, 1.0, 7.0, 4.0, 10.0],
        parcel_indices: &[],
    },
    TestCase {
        label: "duplicate points",
        points: &[3.0, 1.0, 3.0, 2.0, 1.0],
        parcel_indices: &[],
    },
    TestCase {
        label: "parcel set is untouched",
        points: &[5.0, 1.0],
        parcel_indices: &[9, 4],
    },
];

impl ParcelCase for TestCase {
    fn label(&self) -> &'static str {
        self.label
    }

    fn encode_input(&self) -> Result<Vec<f32>, String> {
        if self.points.len() > MAX_PARCEL_INTERVALS {
            return Err(format!(
                "{} inflection points exceeds MAX_PARCELS ({})",
                self.points.len(),
                MAX_PARCEL_INTERVALS
            ));
        }
        if self.parcel_indices.len() > MAX_PARCEL_INTERVALS {
            return Err(format!(
                "{} parcel indices exceeds MAX_PARCELS ({})",
                self.parcel_indices.len(),
                MAX_PARCEL_INTERVALS
            ));
        }

        let mut data = Vec::with_capacity(2 + self.points.len() + self.parcel_indices.len());
        data.push(self.points.len() as f32);
        data.push(self.parcel_indices.len() as f32);
        data.extend_from_slice(self.points);
        data.extend(self.parcel_indices.iter().map(|index| *index as f32));
        Ok(data)
    }

    fn output_floats(&self) -> usize {
        interval_cluster_output_floats()
    }

    fn verify(&self, output: &[f32]) -> Result<(), String> {
        let mut sorted_points = self.points.to_vec();
        sorted_points.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let expected = IntervalCluster {
            size: self.points.len() as u32,
            parcel_count: self.parcel_indices.len() as u32,
            inflection_points: sorted_points,
            parcel_indices: self.parcel_indices.to_vec(),
        };

        verify_interval_cluster(&decode_interval_cluster(output)?, &expected)
    }
}

parcel_test_node!(
    SortIntervalClusterTester,
    CONTEXT,
    TEST_SHADER_PATH,
    TEST_CASES
);
