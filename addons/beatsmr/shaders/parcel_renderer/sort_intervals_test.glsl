#[compute]
#version 450

// ---------------------------------------------------------------------------
// Isolated harness for sort_intervals() from parcel_clusterer.glslinc.
//
// The host uploads an unsorted list of parcel intervals, then checks that
// sort_intervals() orders them by entry point.
//
// Input (binding 0), a flat float array:
//   [0]       interval count
//   then, per interval: entry, exit, parcel_index
//
// Output (binding 1): see write_intervals() in test_harness.glslinc.
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

    sort_intervals(intervals);

    write_intervals(intervals);
}
