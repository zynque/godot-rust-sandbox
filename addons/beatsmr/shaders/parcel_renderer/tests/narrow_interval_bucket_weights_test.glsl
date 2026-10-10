#[compute]
#version 450

// ---------------------------------------------------------------------------
// Isolated harness for narrow_interval_bucket_weights() from
// cluster/interval_bucket_narrowing.glslinc.
//
// The host uploads a ready made set of interval buckets (boundaries, estimated
// positions, evidence, samples and deviations) plus a density threshold. The
// shader drops the leading and trailing buckets whose estimated density, the
// square root of the accumulated squared density, falls below the threshold,
// restratifies the kept span evenly into INTERVAL_BUCKET_COUNT buckets, and
// pools the old estimates into the new buckets, giving any bucket that receives
// nothing the unobserved-bucket prior.
//
// Input (binding 0), a flat float array:
//   the bucket boundaries (INTERVAL_BUCKET_POINTS)
//   then the estimated positions (INTERVAL_BUCKET_COUNT)
//   then the evidence (INTERVAL_BUCKET_COUNT)
//   then the samples (INTERVAL_BUCKET_COUNT)
//   then the deviations (INTERVAL_BUCKET_COUNT)
//   then the density threshold
//
// The bucket set carries no parcels; the search reads only the estimates.
//
// Output (binding 1): see write_bucket_weights() in test_harness.glslinc.
// ---------------------------------------------------------------------------

// Shadertoy style global that the renderer entry point normally declares
// before including parcel_math.glslinc.
vec3 iResolution = vec3(1.0);

#include "res://addons/beatsmr/shaders/parcel_renderer/core/constants.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/core/structs.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/core/globals.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/core/parcel_math.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/cluster/interval_bucket_construction.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/cluster/interval_bucket_observation.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/cluster/interval_bucket_sampling.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/cluster/interval_bucket_narrowing.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/tests/test_harness.glslinc"

void main() {
    uint cursor = 0u;

    ParcelIntervalBucketWeights16 buckets;
    buckets.parcels.size = 0u;

    for (uint i = 0u; i < INTERVAL_BUCKET_POINTS; i++) {
        buckets.buckets[i] = input_data[cursor];
        cursor += 1u;
    }
    for (uint i = 0u; i < INTERVAL_BUCKET_COUNT; i++) {
        buckets.positions[i] = input_data[cursor];
        cursor += 1u;
    }
    for (uint i = 0u; i < INTERVAL_BUCKET_COUNT; i++) {
        buckets.evidence[i] = input_data[cursor];
        cursor += 1u;
    }
    for (uint i = 0u; i < INTERVAL_BUCKET_COUNT; i++) {
        buckets.samples[i] = uint(input_data[cursor]);
        cursor += 1u;
    }
    for (uint i = 0u; i < INTERVAL_BUCKET_COUNT; i++) {
        buckets.deviations[i] = input_data[cursor];
        cursor += 1u;
    }
    float threshold = input_data[cursor];

    ParcelIntervalBucketWeights16 narrowed = narrow_interval_bucket_weights(buckets, threshold);
    write_bucket_weights(narrowed);
}
