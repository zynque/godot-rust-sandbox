use godot::prelude::*;

use crate::parcel_test_common::{
    BucketWeights, INTERVAL_BUCKET_COUNT, INTERVAL_BUCKET_POINTS, Interval, MAX_PARCEL_INTERVALS,
    ParcelCase, bucket_weights_output_floats, decode_bucket_weights, parcel_test_node,
    verify_bucket_weights,
};

// ---------------------------------------------------------------------------
// InitializeIntervalBucketWeightsTester
//
// Compiles the isolated initialize_interval_bucket_weights() compute shader,
// uploads a ray, a parcel buffer and a set of overlapping intervals, dispatches
// a single work-group, and verifies that each interval midpoint becomes the
// estimated position of its bucket, carrying the squared density observed
// there as its evidence, and that buckets with no midpoint take a fraction of
// the average observed evidence as a prior.
// ---------------------------------------------------------------------------

const TEST_SHADER_PATH: &str =
    "res://addons/beatsmr/shaders/parcel_renderer/tests/initialize_interval_bucket_weights_test.glsl";
const CONTEXT: &str = "InitializeIntervalBucketWeightsTester";

/// Mirrors MAX_PARCELS in constants.glslinc.
const MAX_PARCELS: usize = 100;

/// Mirrors INTERVAL_BUCKET_UNOBSERVED_EVIDENCE_FRACTION in constants.glslinc.
const UNOBSERVED_EVIDENCE_FRACTION: f32 = 0.5;

/// The subset of `Parcel` the density evaluation reads. All test parcels are
/// axis aligned, so `inverse_variance` is the diagonal of the inverse
/// covariance; the off-diagonal entries are zero.
#[derive(Clone, Copy)]
struct TestParcel {
    mean: [f32; 3],
    inverse_variance: [f32; 3],
    peak_density: f32,
}

const fn parcel(mean: [f32; 3], inverse_variance: [f32; 3], peak_density: f32) -> TestParcel {
    TestParcel {
        mean,
        inverse_variance,
        peak_density,
    }
}

/// A unit sigma parcel centred on the z axis, so a ray along +z passes through
/// its mean and observes exactly `peak_density` at the mean.
const UNIT_PARCEL_AT_Z_10: TestParcel = parcel([0.0, 0.0, 10.0], [1.0, 1.0, 1.0], 0.8);
const UNIT_PARCEL_AT_Z_6: TestParcel = parcel([0.0, 0.0, 6.0], [1.0, 1.0, 1.0], 0.4);
const UNIT_PARCEL_AT_HALF: TestParcel = parcel([0.0, 0.0, 0.5], [1.0, 1.0, 1.0], 0.4);
const EMPTY_PARCEL_AT_Z_10: TestParcel = parcel([0.0, 0.0, 10.0], [1.0, 1.0, 1.0], 0.0);
const ZERO_PARCEL: TestParcel = parcel([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], 0.0);
const BRIGHT_PARCEL_AT_Z_5: TestParcel = parcel([0.0, 0.0, 5.0], [1.0, 1.0, 1.0], 0.8);
const DIM_PARCEL_AT_Z_10: TestParcel = parcel([0.0, 0.0, 10.0], [1.0, 1.0, 1.0], 0.4);

/// Sixteen intervals whose midpoints (1, 3, ... 31) land one per bucket, so
/// every bucket sees a zero density observation and no bucket gains evidence.
const ALL_BUCKETS_ZERO_DENSITY: &[Interval] = &[
    (0.0, 2.0, 0),
    (2.0, 4.0, 0),
    (4.0, 6.0, 0),
    (6.0, 8.0, 0),
    (8.0, 10.0, 0),
    (10.0, 12.0, 0),
    (12.0, 14.0, 0),
    (14.0, 16.0, 0),
    (16.0, 18.0, 0),
    (18.0, 20.0, 0),
    (20.0, 22.0, 0),
    (22.0, 24.0, 0),
    (24.0, 26.0, 0),
    (26.0, 28.0, 0),
    (28.0, 30.0, 0),
    (30.0, 32.0, 0),
];

struct TestCase {
    label: &'static str,
    ray_origin: [f32; 3],
    ray_direction: [f32; 3],
    parcels: &'static [TestParcel],
    /// (entry, exit, parcel_index) per interval, in upload order.
    intervals: &'static [Interval],
}

const TEST_CASES: &[TestCase] = &[
    TestCase {
        label: "empty",
        ray_origin: [0.0, 0.0, 0.0],
        ray_direction: [0.0, 0.0, 1.0],
        parcels: &[],
        intervals: &[],
    },
    TestCase {
        label: "midpoint at the parcel mean observes the peak density",
        ray_origin: [0.0, 0.0, 0.0],
        ray_direction: [0.0, 0.0, 1.0],
        parcels: &[UNIT_PARCEL_AT_Z_10],
        intervals: &[(7.0, 13.0, 0)],
    },
    TestCase {
        label: "two midpoints seed two buckets",
        ray_origin: [0.0, 0.0, 0.0],
        ray_direction: [0.0, 0.0, 1.0],
        parcels: &[UNIT_PARCEL_AT_Z_10, UNIT_PARCEL_AT_Z_6],
        intervals: &[(7.0, 13.0, 0), (3.0, 9.0, 1)],
    },
    TestCase {
        label: "duplicate midpoints count one observed bucket",
        ray_origin: [0.0, 0.0, 0.0],
        ray_direction: [0.0, 0.0, 1.0],
        parcels: &[BRIGHT_PARCEL_AT_Z_5, DIM_PARCEL_AT_Z_10],
        intervals: &[(0.0, 1.0, 0), (0.0, 1.0, 1), (10.0, 14.0, 1)],
    },
    TestCase {
        label: "two midpoints in the same bucket keep the last",
        ray_origin: [0.0, 0.0, 0.0],
        ray_direction: [0.0, 0.0, 1.0],
        parcels: &[UNIT_PARCEL_AT_HALF, BRIGHT_PARCEL_AT_Z_5],
        intervals: &[(0.0, 1.0, 0), (0.0, 1.0, 1)],
    },
    TestCase {
        label: "zero observed density seeds no evidence",
        ray_origin: [0.0, 0.0, 0.0],
        ray_direction: [0.0, 0.0, 1.0],
        parcels: &[EMPTY_PARCEL_AT_Z_10],
        intervals: &[(7.0, 13.0, 0)],
    },
    TestCase {
        label: "midpoint on the far boundary lands in the last bucket",
        ray_origin: [0.0, 0.0, 0.0],
        ray_direction: [0.0, 0.0, 1.0],
        parcels: &[BRIGHT_PARCEL_AT_Z_5, DIM_PARCEL_AT_Z_10],
        intervals: &[(0.0, 10.0, 0), (10.0, 10.0, 1)],
    },
    TestCase {
        label: "every bucket observes zero density",
        ray_origin: [0.0, 0.0, 0.0],
        ray_direction: [0.0, 0.0, 1.0],
        parcels: &[ZERO_PARCEL],
        intervals: ALL_BUCKETS_ZERO_DENSITY,
    },
];

/// Total density the shader should observe at point `p`.
fn total_density(p: [f32; 3], parcels: &[TestParcel], indices: &[u32]) -> f32 {
    let mut density = 0.0f32;
    for index in indices {
        let parcel = &parcels[*index as usize];
        let dx = p[0] - parcel.mean[0];
        let dy = p[1] - parcel.mean[1];
        let dz = p[2] - parcel.mean[2];
        let mahalanobis_squared = parcel.inverse_variance[0] * dx * dx
            + parcel.inverse_variance[1] * dy * dy
            + parcel.inverse_variance[2] * dz * dz;
        density += parcel.peak_density * (-0.5 * mahalanobis_squared).exp();
    }
    density
}

/// Index of the bucket containing `t`, mirroring find_interval_bucket().
fn find_bucket(buckets: &[f32], t: f32) -> Option<usize> {
    for i in 0..INTERVAL_BUCKET_COUNT {
        let is_last = i == INTERVAL_BUCKET_COUNT - 1;
        let inside = t >= buckets[i] && (t < buckets[i + 1] || (is_last && t <= buckets[i + 1]));
        if inside {
            return Some(i);
        }
    }
    None
}

/// The buckets `initialize_interval_bucket_weights()` should produce for a case.
fn expected_bucket_weights(case: &TestCase) -> BucketWeights {
    let parcel_indices: Vec<u32> = case
        .intervals
        .iter()
        .map(|(_, _, parcel_index)| *parcel_index as u32)
        .collect();

    let mut buckets = vec![0.0f32; INTERVAL_BUCKET_POINTS];
    let mut positions = vec![0.0f32; INTERVAL_BUCKET_COUNT];
    let mut evidence = vec![0.0f32; INTERVAL_BUCKET_COUNT];
    let mut samples = vec![0u32; INTERVAL_BUCKET_COUNT];
    let mut observed = vec![false; INTERVAL_BUCKET_COUNT];

    if let Some((first_entry, first_exit, _)) = case.intervals.first() {
        let mut front = *first_entry;
        let mut back = *first_exit;
        for (entry, exit, _) in case.intervals {
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

        for (entry, exit, _) in case.intervals {
            let t = 0.5 * (entry + exit);
            let Some(bucket) = find_bucket(&buckets, t) else {
                continue;
            };

            let p = [
                case.ray_origin[0] + t * case.ray_direction[0],
                case.ray_origin[1] + t * case.ray_direction[1],
                case.ray_origin[2] + t * case.ray_direction[2],
            ];
            let density = total_density(p, case.parcels, &parcel_indices);

            positions[bucket] = t;
            evidence[bucket] = density * density;
            samples[bucket] = 1;
            observed[bucket] = true;
        }

        let observed_count = observed.iter().filter(|seen| **seen).count();
        if observed_count > 0 {
            let observed_evidence: f32 = (0..INTERVAL_BUCKET_COUNT)
                .filter(|i| observed[*i])
                .map(|i| evidence[i])
                .sum();
            let prior =
                UNOBSERVED_EVIDENCE_FRACTION * observed_evidence / observed_count as f32;
            for i in 0..INTERVAL_BUCKET_COUNT {
                if observed[i] {
                    continue;
                }
                positions[i] = 0.5 * (buckets[i] + buckets[i + 1]);
                evidence[i] = prior;
            }
        }
    }

    BucketWeights {
        parcel_count: case.intervals.len() as u32,
        buckets,
        positions,
        evidence,
        samples,
        deviations: vec![0.0; INTERVAL_BUCKET_COUNT],
        parcel_indices,
    }
}

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
        if self.intervals.len() > MAX_PARCEL_INTERVALS {
            return Err(format!(
                "{} intervals exceeds MAX_PARCEL_INTERVALS ({})",
                self.intervals.len(),
                MAX_PARCEL_INTERVALS
            ));
        }
        for (_, _, parcel_index) in self.intervals {
            if *parcel_index as usize >= self.parcels.len() {
                return Err(format!(
                    "interval parcel index {} is out of range for {} parcels",
                    parcel_index,
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
            data.push(parcel.peak_density);
        }

        data.push(self.intervals.len() as f32);
        for (entry, exit, parcel_index) in self.intervals {
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
            &expected_bucket_weights(self),
        )
    }
}

parcel_test_node!(
    InitializeIntervalBucketWeightsTester,
    CONTEXT,
    TEST_SHADER_PATH,
    TEST_CASES
);
