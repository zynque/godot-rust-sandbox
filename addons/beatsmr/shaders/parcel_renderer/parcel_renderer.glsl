#[compute]
#version 450

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;
layout(set = 0, binding = 0, rgba32f) uniform image2D output_image;

layout(set = 0, binding = 1, std140) uniform TimeParams {
	vec4 t;
} time_params;

vec3 iResolution = vec3(1.0);
float iTime = 0.0;

// Modular parcel renderer assembled by GLSL includes.
#include "res://addons/beatsmr/shaders/parcel_renderer/constants.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/structs.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/parcel_math.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/intervals.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/tracing.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/scene.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/main_image.glslinc"

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
