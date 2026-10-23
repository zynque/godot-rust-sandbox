#[compute]
#version 450

#ifndef MAX_PARCELS
#define MAX_PARCELS 100
#endif
#ifndef MAX_PARCEL_INTERVALS
#define MAX_PARCEL_INTERVALS MAX_PARCELS
#endif
#ifndef INTERVAL_BUCKET_COUNT
#define INTERVAL_BUCKET_COUNT 16
#endif
#ifndef INTERVAL_BUCKET_POINTS
#define INTERVAL_BUCKET_POINTS 17
#endif
#ifndef MAX_CLUSTER
#define MAX_CLUSTER 10
#endif

// ---------------------------------------------------------------------------
// Isolated harness for sort_interval_cluster() from
// cluster/interval_cluster.glslinc.
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
