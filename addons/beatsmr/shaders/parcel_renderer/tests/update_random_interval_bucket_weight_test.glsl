#[compute]
#version 450

// ---------------------------------------------------------------------------
// Isolated harness for update_random_interval_bucket_weight() from
// cluster/interval_buckets.glslinc.
//
// The host uploads a ray, a parcel buffer, a list of overlapping intervals and
// a seed. The shader seeds the buckets from the interval midpoints
// (initialize_interval_bucket_weights()), leaving buckets with no midpoint
// holding a prior evidence, then refines a single bucket: it picks one at
// random with probability proportional to its weight, draws a random position
// inside it, samples the total density at the matching ray point and folds that
// observation, weighted by its squared density, into its estimate.
//
// Input (binding 0), a flat float array:
//   [0 .. 3)  ray.origin
//   [3 .. 6)  ray.direction (normalized)
//   [6]       parcel count
//   then, per parcel: mean (3), the inverse covariance (9, column major) and
//   the peak density
//   then the interval count, followed by entry, exit and parcel_index per
//   interval
//   then the seed
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
#include "res://addons/beatsmr/shaders/parcel_renderer/cluster/parcel_intervals.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/cluster/interval_cluster.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/cluster/interval_bucket_construction.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/cluster/interval_bucket_observation.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/cluster/interval_bucket_sampling.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/tests/test_harness.glslinc"

const uint PARCEL_STRIDE = 13u;

void main() {
    uint cursor = 0u;

    Ray ray;
    ray.origin = vec3(input_data[cursor], input_data[cursor + 1u], input_data[cursor + 2u]);
    cursor += 3u;
    ray.direction = vec3(input_data[cursor], input_data[cursor + 1u], input_data[cursor + 2u]);
    cursor += 3u;

    uint parcel_count = uint(input_data[cursor]);
    cursor += 1u;

    for (uint p = 0u; p < parcel_count; p++) {
        uint base = cursor + p * PARCEL_STRIDE;

        Parcel parcel;
        parcel.mean = vec3(input_data[base], input_data[base + 1u], input_data[base + 2u]);
        parcel.covariance = mat3(0.0);
        parcel.inverseCovariance = mat3(
            input_data[base + 3u], input_data[base + 4u], input_data[base + 5u],
            input_data[base + 6u], input_data[base + 7u], input_data[base + 8u],
            input_data[base + 9u], input_data[base + 10u], input_data[base + 11u]
        );
        parcel.peakDensity = input_data[base + 12u];
        parcel.color = vec3(0.0);

        parcel_buffer[p] = parcel;
    }
    cursor += parcel_count * PARCEL_STRIDE;

    ParcelIntervals intervals;
    intervals.size = uint(input_data[cursor]);
    cursor += 1u;

    for (uint i = 0u; i < intervals.size; i++) {
        ParcelInterval interval;
        interval.entry = input_data[cursor];
        interval.exit = input_data[cursor + 1u];
        interval.parcel_index = int(input_data[cursor + 2u]);
        intervals.items[i] = interval;
        cursor += 3u;
    }

    uint seed = uint(input_data[cursor]);

    ParcelIntervalBucketWeights16 buckets = make_interval_buckets(intervals);
    initialize_interval_bucket_weights(buckets, intervals, ray);
    update_random_interval_bucket_weight(buckets, ray, seed);

    write_bucket_weights(buckets);
}
