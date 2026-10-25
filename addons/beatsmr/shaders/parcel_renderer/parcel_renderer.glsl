#[compute]
#version 450

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;
layout(set = 0, binding = 0, rgba32f) uniform image2D output_image;

layout(set = 0, binding = 1, std140) uniform TimeParams {
	vec4 t;
} time_params;

layout(set = 0, binding = 2, std140) uniform CameraParams {
    mat4 view;
    mat4 projection;
} camera_params;

vec3 iResolution = vec3(1.0);
float iTime = 0.0;


// Modular parcel renderer assembled by GLSL includes.
// Include order matters: each file may use definitions from the files above it.
// A pixel's ray is intersected against the parcel set, its intervals are grouped
// into clusters, and each cluster is searched adaptively for the first density
// threshold crossing that marks a solid surface
// (see cluster/surface_search.glslinc).
// NOTE: Godot only reimports this shader when this file changes, not when an
// included .glslinc changes, so reimport it after editing any include.
#include "res://addons/beatsmr/shaders/parcel_renderer/core/constants.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/core/structs.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/core/globals.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/core/parcel_math.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/cluster/parcel_intervals.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/cluster/surface_search.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/render/intervals.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/render/tracing.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/render/main_image.glslinc"

void main() {
	ivec2 pixel = ivec2(gl_GlobalInvocationID.xy);
	ivec2 size = imageSize(output_image);

	if (pixel.x >= size.x || pixel.y >= size.y) {
		return;
	}

	iResolution = vec3(float(size.x), float(size.y), 1.0);
	iTime = time_params.t.x;

	vec4 fragColor = vec4(0.0);
	mainImage(fragColor, vec2(pixel) + vec2(0.5));
	imageStore(output_image, pixel, fragColor);
}
