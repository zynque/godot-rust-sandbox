#[compute]
#version 450

// ---------------------------------------------------------------------------
// Isolated harness for build_interval_cluster() from
// cluster/interval_cluster.glslinc.
//
// The host uploads a list of parcel intervals and checks the cluster that
// build_interval_cluster() derives from them.
//
// Input (binding 0), a flat float array:
//   [0]       interval count
//   then, per interval: entry, exit, parcel_index
//
// Output (binding 1), a flat float array:
//   [0]       cluster.size (inflection points, two per interval)
//   [1]       cluster.parcels.size
//   then cluster.inflection_points[0 .. cluster.size)
//   then cluster.parcels.indices[0 .. cluster.parcels.size)
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
#include "res://addons/beatsmr/shaders/parcel_renderer/cluster/interval_buckets.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/tests/test_harness.glslinc"

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

    ParcelIntervalCluster cluster = build_interval_cluster(intervals);

    write_interval_cluster(cluster);
}
