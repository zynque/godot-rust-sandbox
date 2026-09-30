use godot::prelude::*;

use crate::parcel_test_common::{Interval, MAX_PARCEL_INTERVALS, ParcelCase, parcel_test_node};

// ---------------------------------------------------------------------------
// CountOverlappingIntervalsTester
//
// Compiles the isolated count_overlapping_intervals() compute shader, uploads a
// set of intervals sorted by entry plus a start index, dispatches a single
// work-group, and verifies how many consecutive intervals from that index each
// overlap their successor.
// ---------------------------------------------------------------------------

const TEST_SHADER_PATH: &str =
    "res://addons/beatsmr/shaders/parcel_renderer/tests/count_overlapping_intervals_test.glsl";
const CONTEXT: &str = "CountOverlappingIntervalsTester";

struct TestCase {
    label: &'static str,
    /// Intervals sorted by entry, in the order they are uploaded.
    input: &'static [Interval],
    start_index: u32,
    expected: u32,
}

const TEST_CASES: &[TestCase] = &[
    TestCase {
        label: "empty",
        input: &[],
        start_index: 0,
        expected: 0,
    },
    TestCase {
        label: "start index outside the set",
        input: &[(1.0, 2.0, 0)],
        start_index: 1,
        expected: 0,
    },
    TestCase {
        label: "single interval",
        input: &[(5.0, 10.0, 0)],
        start_index: 0,
        expected: 1,
    },
    TestCase {
        label: "disjoint intervals",
        input: &[(1.0, 2.0, 0), (3.0, 4.0, 1)],
        start_index: 0,
        expected: 1,
    },
    TestCase {
        label: "touching intervals do not overlap",
        input: &[(1.0, 4.0, 0), (4.0, 8.0, 1)],
        start_index: 0,
        expected: 1,
    },
    TestCase {
        label: "run stops at the first gap",
        input: &[(1.0, 4.0, 0), (3.0, 6.0, 1), (10.0, 12.0, 2)],
        start_index: 0,
        expected: 2,
    },
    TestCase {
        label: "every interval overlaps the next",
        input: &[(1.0, 10.0, 0), (2.0, 9.0, 1), (3.0, 8.0, 2)],
        start_index: 0,
        expected: 3,
    },
    TestCase {
        label: "start index in the middle of a run",
        input: &[
            (1.0, 10.0, 0),
            (2.0, 9.0, 1),
            (3.0, 20.0, 2),
            (4.0, 21.0, 3),
        ],
        start_index: 2,
        expected: 2,
    },
    TestCase {
        label: "start index on the last interval",
        input: &[(1.0, 4.0, 0), (3.0, 8.0, 1)],
        start_index: 1,
        expected: 1,
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

        let mut data = Vec::with_capacity(2 + 3 * self.input.len());
        data.push(self.input.len() as f32);
        for (entry, exit, parcel_index) in self.input {
            data.extend_from_slice(&[*entry, *exit, *parcel_index as f32]);
        }
        data.push(self.start_index as f32);
        Ok(data)
    }

    fn output_floats(&self) -> usize {
        1
    }

    fn verify(&self, output: &[f32]) -> Result<(), String> {
        let actual = output[0].round() as u32;
        if actual == self.expected {
            Ok(())
        } else {
            Err(format!("expected count {}, got {}", self.expected, actual))
        }
    }
}

parcel_test_node!(
    CountOverlappingIntervalsTester,
    CONTEXT,
    TEST_SHADER_PATH,
    TEST_CASES
);
