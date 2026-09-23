#[compute]
#version 450

// ---------------------------------------------------------------------------
// Isolated harness for intersect_parcels() from parcel_clusterer.glslinc.
//
// The host uploads a ray, a parcel buffer and a set of parcel indices, then
// checks the boundary intervals that intersect_parcels() reports.
//
// Input (binding 0), a flat float array:
//   [0 .. 3)  ray.origin
//   [3 .. 6)  ray.direction (normalized)
//   [6]       parcel count
//   then, per parcel: mean (3) followed by the inverse covariance (9, column
//   major)
//   then the set size, followed by that many parcel indices
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

const uint PARCEL_STRIDE = 12u;

void main() {
    uint cursor = 0u;

    Ray ray;
    ray.origin = vec3(input_data[cursor], input_data[cursor + 1u], input_data[cursor + 2u]);
    cursor += 3u;
    ray.direction = vec3(input_data[cursor], input_data[cursor + 1u], input_data[cursor + 2u]);
    cursor += 3u;

    uint parcel_count = uint(input_data[cursor]);
    cursor += 1u;

    // intersect_parcels() only reads a parcel's mean and inverse covariance,
    // so the remaining fields are left empty.
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
        parcel.peakDensity = 0.0;
        parcel.color = vec3(0.0);

        parcel_buffer[p] = parcel;
    }
    cursor += parcel_count * PARCEL_STRIDE;

    ParcelSet set;
    set.size = uint(input_data[cursor]);
    cursor += 1u;
    for (uint i = 0u; i < set.size; i++) {
        set.indices[i] = uint(input_data[cursor + i]);
    }

    ParcelIntervals intervals = intersect_parcels(ray, set);

    write_intervals(intervals);
}
