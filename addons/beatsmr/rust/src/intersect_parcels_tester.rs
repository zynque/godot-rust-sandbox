use godot::prelude::*;

use crate::parcel_test_common::{
    Interval, ParcelCase, decode_intervals, interval_output_floats, parcel_test_node,
    verify_intervals,
};

// ---------------------------------------------------------------------------
// IntersectParcelsTester
//
// Compiles the isolated intersect_parcels() compute shader, uploads a ray, a
// parcel buffer and a set of parcel indices, dispatches a single work-group,
// and verifies the reported ray/parcel boundary intervals.
// ---------------------------------------------------------------------------

const TEST_SHADER_PATH: &str =
    "res://addons/beatsmr/shaders/parcel_renderer/intersect_parcels_test.glsl";
const CONTEXT: &str = "IntersectParcelsTester";

/// Mirrors MAX_PARCELS in constants.glslinc.
const MAX_PARCELS: usize = 100;

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
    expected: &'static [Interval],
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

impl ParcelCase for TestCase {
    fn label(&self) -> &'static str {
        self.label
    }

    fn encode_input(&self) -> Result<Vec<f32>, String> {
        if self.parcels.len() > MAX_PARCELS {
            return Err(format!(
                "{} parcels exceeds MAX_PARCELS ({})",
                self.parcels.len(),
                MAX_PARCELS
            ));
        }
        if self.set_indices.len() > MAX_PARCELS {
            return Err(format!(
                "set of {} indices exceeds MAX_PARCELS ({})",
                self.set_indices.len(),
                MAX_PARCELS
            ));
        }
        for index in self.set_indices {
            if *index as usize >= self.parcels.len() {
                return Err(format!(
                    "set index {} is out of range for {} parcels",
                    index,
                    self.parcels.len()
                ));
            }
        }

        let mut data = Vec::new();
        data.extend_from_slice(&self.ray_origin);
        data.extend_from_slice(&self.ray_direction);

        data.push(self.parcels.len() as f32);
        for parcel in self.parcels {
            let [x, y, z] = parcel.inverse_variance;
            data.extend_from_slice(&parcel.mean);
            // Column major diagonal.
            data.extend_from_slice(&[x, 0.0, 0.0, 0.0, y, 0.0, 0.0, 0.0, z]);
        }

        data.push(self.set_indices.len() as f32);
        data.extend(self.set_indices.iter().map(|index| *index as f32));

        Ok(data)
    }

    fn output_floats(&self) -> usize {
        interval_output_floats()
    }

    fn verify(&self, output: &[f32]) -> Result<(), String> {
        verify_intervals(&decode_intervals(output)?, self.expected)
    }
}

parcel_test_node!(
    IntersectParcelsTester,
    CONTEXT,
    TEST_SHADER_PATH,
    TEST_CASES
);
