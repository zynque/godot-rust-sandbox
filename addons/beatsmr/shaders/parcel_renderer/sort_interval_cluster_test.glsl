#[compute]
#version 450

// ---------------------------------------------------------------------------
// Isolated harness for sort_interval_cluster() from parcel_clusterer.glslinc.
//
// The host uploads a cluster and checks the sorted inflection points.
//
// Input (binding 0), a flat float array:
//   [0]       cluster.size
//   [1]       cluster.parcels.size
//   then cluster.inflection_points[0 .. cluster.size)
//   then cluster.parcels.indices[0 .. cluster.parcels.size)
//
// Output (binding 1): the same layout, via write_interval_cluster() in
// test_harness.glslinc.
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

    ParcelIntervalCluster cluster;
    cluster.size = uint(input_data[cursor]);
    cursor += 1u;
    cluster.parcels.size = uint(input_data[cursor]);
    cursor += 1u;

    for (uint i = 0u; i < cluster.size; i++) {
        cluster.inflection_points[i] = input_data[cursor];
        cursor += 1u;
    }
    for (uint i = 0u; i < cluster.parcels.size; i++) {
        cluster.parcels.indices[i] = uint(input_data[cursor]);
        cursor += 1u;
    }

    sort_interval_cluster(cluster);

    write_interval_cluster(cluster);
}
