use godot::prelude::*;

use crate::parcel_test_common::{
    INTERVAL_BUCKET_COUNT, INTERVAL_BUCKET_POINTS, ParcelCase, parcel_test_node,
};

// ---------------------------------------------------------------------------
// SearchIntervalBucketSurfaceTester
//
// Compiles the isolated search_interval_bucket_surface() compute shader,
// uploads a ray, a parcel buffer and the bucket boundaries that span the range
// the stochastic phase narrowed to, dispatches a single work-group, and
// verifies that the exact density scan refines the first threshold crossing to
// the surface with the guarded Newton solve, and reports the densest sample
// when no crossing is found.
// ---------------------------------------------------------------------------

const TEST_SHADER_PATH: &str =
    "res://addons/beatsmr/shaders/parcel_renderer/tests/search_interval_bucket_surface_test.glsl";
const CONTEXT: &str = "SearchIntervalBucketSurfaceTester";

/// Mirrors MAX_PARCELS in constants.glslinc.
const MAX_PARCELS: usize = 100;
/// Mirrors ISOSURFACE in constants.glslinc.
const ISOSURFACE: f32 = 0.5;
/// Mirrors SURFACE_NEWTON_ITERATIONS in constants.glslinc.
const SURFACE_NEWTON_ITERATIONS: usize = 8;
/// Mirrors SURFACE_RESIDUAL_EPSILON in constants.glslinc.
const SURFACE_RESIDUAL_EPSILON: f32 = 1e-5;
/// Mirrors SURFACE_SLOPE_EPSILON in constants.glslinc.
const SURFACE_SLOPE_EPSILON: f32 = 1e-6;
/// Mirrors INTERVAL_SURFACE_SCAN_STEP in constants.glslinc.
const SURFACE_SCAN_STEP: f32 = 0.005;
/// Mirrors INTERVAL_SURFACE_SCAN_MAX_STEPS in constants.glslinc.
const SURFACE_SCAN_MAX_STEPS: f32 = 512.0;

/// The guarded Newton solve can stop a pass apart between the GPU and this
/// mirror, so the settled position and density are compared more loosely than
/// the shared EPSILON.
const SURFACE_EPSILON: f32 = 1e-3;

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

/// A unit sigma parcel whose density is exactly `peak_density` at its mean, so
/// a ray along +z through the mean observes a known gaussian profile.
const UNIT_PARCEL_AT_Z_10: TestParcel = parcel([0.0, 0.0, 10.0], [1.0, 1.0, 1.0], 1.0);
const UNIT_PARCEL_AT_Z_10_6: TestParcel = parcel([0.0, 0.0, 10.6], [1.0, 1.0, 1.0], 1.0);
/// A narrow parcel peaked below ISOSURFACE, so the scan finds no crossing and
/// the densest sample is unambiguous.
const DIM_PARCEL_AT_Z_10_1: TestParcel = parcel([0.0, 0.0, 10.1], [1.0, 1.0, 400.0], 0.3);

struct TestCase {
    label: &'static str,
    ray_origin: [f32; 3],
    ray_direction: [f32; 3],
    parcels: &'static [TestParcel],
    /// The bucket extent along the ray. The boundaries are uploaded evenly
    /// stratified across it, matching uniform_boundaries().
    front: f32,
    back: f32,
}

const TEST_CASES: &[TestCase] = &[
    TestCase {
        label: "empty parcel set",
        ray_origin: [0.0, 0.0, 0.0],
        ray_direction: [0.0, 0.0, 1.0],
        parcels: &[],
        front: 0.0,
        back: 0.0,
    },
    TestCase {
        label: "crossing refined to a gaussian surface",
        ray_origin: [0.0, 0.0, 0.0],
        ray_direction: [0.0, 0.0, 1.0],
        parcels: &[UNIT_PARCEL_AT_Z_10],
        front: 8.0,
        back: 12.0,
    },
    TestCase {
        label: "no crossing returns the densest sample",
        ray_origin: [0.0, 0.0, 0.0],
        ray_direction: [0.0, 0.0, 1.0],
        parcels: &[DIM_PARCEL_AT_Z_10_1],
        front: 8.0,
        back: 12.0,
    },
    TestCase {
        label: "overlapping parcels raise the crossing",
        ray_origin: [0.0, 0.0, 0.0],
        ray_direction: [0.0, 0.0, 1.0],
        parcels: &[UNIT_PARCEL_AT_Z_10, UNIT_PARCEL_AT_Z_10_6],
        front: 8.0,
        back: 12.0,
    },
];

/// Mirrors the evenly stratified boundaries make_interval_buckets() builds for
/// a cluster spanning `front` to `back`.
fn uniform_boundaries(front: f32, back: f32) -> [f32; INTERVAL_BUCKET_POINTS] {
    let mut boundaries = [0.0f32; INTERVAL_BUCKET_POINTS];
    let span = back - front;
    for (i, boundary) in boundaries.iter_mut().enumerate() {
        *boundary = front + span * i as f32 / INTERVAL_BUCKET_COUNT as f32;
    }
    boundaries
}

fn ray_point(case: &TestCase, t: f32) -> [f32; 3] {
    [
        case.ray_origin[0] + t * case.ray_direction[0],
        case.ray_origin[1] + t * case.ray_direction[1],
        case.ray_origin[2] + t * case.ray_direction[2],
    ]
}

/// Total density the shader should observe at point `p`, summing every parcel.
fn total_density(p: [f32; 3], parcels: &[TestParcel]) -> f32 {
    let mut density = 0.0f32;
    for parcel in parcels {
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

/// Mirrors total_parcel_density_gradient() in
/// cluster/interval_bucket_construction.glslinc.
fn total_density_gradient(p: [f32; 3], parcels: &[TestParcel]) -> [f32; 3] {
    let mut gradient = [0.0f32; 3];
    for parcel in parcels {
        let d = [
            p[0] - parcel.mean[0],
            p[1] - parcel.mean[1],
            p[2] - parcel.mean[2],
        ];
        let mahalanobis_squared = parcel.inverse_variance[0] * d[0] * d[0]
            + parcel.inverse_variance[1] * d[1] * d[1]
            + parcel.inverse_variance[2] * d[2] * d[2];
        let density = parcel.peak_density * (-0.5 * mahalanobis_squared).exp();
        for axis in 0..3 {
            gradient[axis] += -density * parcel.inverse_variance[axis] * d[axis];
        }
    }
    gradient
}

/// Mirrors refine_surface_distance() in cluster/surface_search.glslinc.
fn refine_surface_distance(case: &TestCase, mut lo: f32, mut hi: f32) -> f32 {
    let mut t = 0.5 * (lo + hi);
    for _ in 0..SURFACE_NEWTON_ITERATIONS {
        let p = ray_point(case, t);
        let residual = total_density(p, case.parcels) - ISOSURFACE;
        if residual.abs() < SURFACE_RESIDUAL_EPSILON {
            break;
        }

        if residual > 0.0 {
            hi = t;
        } else {
            lo = t;
        }

        let gradient = total_density_gradient(p, case.parcels);
        let slope = gradient[0] * case.ray_direction[0]
            + gradient[1] * case.ray_direction[1]
            + gradient[2] * case.ray_direction[2];
        let step = if slope.abs() > SURFACE_SLOPE_EPSILON {
            t - residual / slope
        } else {
            0.5 * (lo + hi)
        };
        t = if step > lo && step < hi {
            step
        } else {
            0.5 * (lo + hi)
        };
    }
    t
}

/// The (position, density, isSurface) search_interval_bucket_surface() should
/// return for a case: walk the span at the scan resolution and refine the first
/// rising crossing, or report the densest sample when none is found.
fn expected_surface(case: &TestCase) -> (f32, f32, bool) {
    if case.parcels.is_empty() {
        return (0.0, 0.0, false);
    }

    let front = case.front;
    let span = case.back - case.front;
    let count = (span / SURFACE_SCAN_STEP)
        .ceil()
        .clamp(1.0, SURFACE_SCAN_MAX_STEPS);
    let step = span / count;
    let steps = count as i32;

    let front_density = total_density(ray_point(case, front), case.parcels);
    if front_density > ISOSURFACE {
        return (front, front_density, true);
    }

    let mut previous = front;
    let mut densest_position = front;
    let mut densest_density = front_density;

    for i in 1..=steps {
        let t = front + step * i as f32;
        let density = total_density(ray_point(case, t), case.parcels);
        if density > densest_density {
            densest_density = density;
            densest_position = t;
        }
        if density > ISOSURFACE {
            let position = refine_surface_distance(case, previous, t);
            let refined = total_density(ray_point(case, position), case.parcels);
            return (position, refined, true);
        }
        previous = t;
    }

    (densest_position, densest_density, false)
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

        data.extend_from_slice(&uniform_boundaries(self.front, self.back));
        Ok(data)
    }

    fn output_floats(&self) -> usize {
        3
    }

    fn verify(&self, output: &[f32]) -> Result<(), String> {
        let (position, density, is_surface) = expected_surface(self);
        let surface = output[2] > 0.5;
        let matches = (output[0] - position).abs() <= SURFACE_EPSILON
            && (output[1] - density).abs() <= SURFACE_EPSILON
            && surface == is_surface;

        if matches {
            Ok(())
        } else {
            Err(format!(
                "expected (position {position:.6}, density {density:.6}, surface {is_surface}), \
                 got (position {:.6}, density {:.6}, surface {surface})",
                output[0], output[1]
            ))
        }
    }
}

parcel_test_node!(
    SearchIntervalBucketSurfaceTester,
    CONTEXT,
    TEST_SHADER_PATH,
    TEST_CASES
);
