#[compute]
#version 450

// ---------------------------------------------------------------------------
// Isolated harness for search_interval_bucket_surface() from
// cluster/surface_search.glslinc.
//
// The host uploads a ray, a parcel buffer and the bucket boundaries that span
// the range the stochastic phase narrowed to. The shader walks that span with
// exact density samples, refines the first crossing it finds with the guarded
// Newton solve, and otherwise reports the densest sample.
//
// Input (binding 0), a flat float array:
//   [0 .. 3)  ray.origin
//   [3 .. 6)  ray.direction (normalized)
//   [6]       parcel count
//   then, per parcel: mean (3), the inverse covariance (9, column major) and
//   the peak density
//   then the bucket boundaries (INTERVAL_BUCKET_POINTS)
//
// The bucket parcel set is the uploaded parcel buffer in order. The estimated
// positions and evidence are not read by the search and stay zero.
//
// Output (binding 1): see write_surface_sample() in test_harness.glslinc.
// ---------------------------------------------------------------------------

// Shadertoy style global that the renderer entry point normally declares
// before including parcel_math.glslinc.
vec3 iResolution = vec3(1.0);

#include "res://addons/beatsmr/shaders/parcel_renderer/core/constants.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/core/structs.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/core/globals.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/core/parcel_math.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/cluster/interval_bucket_construction.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/cluster/surface_search.glslinc"
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

    ParcelIntervalBucketWeights16 buckets;
    buckets.parcels.size = parcel_count;
    for (uint i = 0u; i < parcel_count; i++)
        buckets.parcels.indices[i] = i;

    for (uint i = 0u; i < INTERVAL_BUCKET_POINTS; i++) {
        buckets.buckets[i] = input_data[cursor];
        cursor += 1u;
    }
    for (uint i = 0u; i < INTERVAL_BUCKET_COUNT; i++) {
        buckets.positions[i] = 0.0;
        buckets.evidence[i] = 0.0;
        buckets.samples[i] = 0u;
        buckets.deviations[i] = 0.0;
    }

    SurfaceSample result = search_interval_bucket_surface(buckets, ray);
    write_surface_sample(result);
}
