#[compute]
#version 450

// ---------------------------------------------------------------------------
// Isolated harness for make_interval_buckets() from parcel_clusterer.glslinc.
//
// The host uploads a list of parcel intervals and checks the evenly spaced
// bucket boundaries and even starting weights that make_interval_buckets()
// derives from them.
//
// Input (binding 0), a flat float array:
//   [0]       interval count
//   then, per interval: entry, exit, parcel_index
//
// Output (binding 1): see write_bucket_weights() in test_harness.glslinc.
// ---------------------------------------------------------------------------

// Shadertoy style global that the renderer entry point normally declares
// before including parcel_math.glslinc.
vec3 iResolution = vec3(1.0);

#include "res://addons/beatsmr/shaders/parcel_renderer/constants.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/structs.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/globals.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/parcel_math.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/parcel_clusterer.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/test_harness.glslinc"

void main() {
    uint cursor = 0u;

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

    ParcelIntervalBucketWeights16 buckets = make_interval_buckets(intervals);

    write_bucket_weights(buckets);
}
