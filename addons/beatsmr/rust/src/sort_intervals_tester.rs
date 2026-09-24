use godot::prelude::*;

use crate::parcel_test_common::{
    Interval, MAX_PARCEL_INTERVALS, ParcelCase, decode_intervals, interval_output_floats,
    parcel_test_node, verify_intervals,
};

// ---------------------------------------------------------------------------
// SortIntervalsTester
//
// Compiles the isolated sort_intervals() compute shader, uploads an unsorted
// list of parcel intervals, dispatches a single work-group, and verifies that
// the intervals come back ordered by entry.
// ---------------------------------------------------------------------------

const TEST_SHADER_PATH: &str =
    "res://addons/beatsmr/shaders/parcel_renderer/sort_intervals_test.glsl";
const CONTEXT: &str = "SortIntervalsTester";

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
        interval_output_floats()
    }

    fn verify(&self, output: &[f32]) -> Result<(), String> {
        verify_intervals(&decode_intervals(output)?, self.expected)
    }
}

parcel_test_node!(SortIntervalsTester, CONTEXT, TEST_SHADER_PATH, TEST_CASES);
