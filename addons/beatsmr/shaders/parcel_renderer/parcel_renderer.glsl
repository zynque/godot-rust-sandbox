#[fragment]
#version 450

// Shadertoy-style globals mapped to an RD uniform block.
layout(set = 0, binding = 0, std140) uniform ShaderToyParams {
	vec3 iResolution;
	float iTime;
} shadertoy;

#define iResolution shadertoy.iResolution
#define iTime shadertoy.iTime

layout(location = 0) out vec4 out_color;

// Modular parcel renderer assembled by GLSL includes.
#include "res://addons/beatsmr/shaders/parcel_renderer/constants.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/structs.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/parcel_math.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/intervals.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/tracing.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/scene.glslinc"
#include "res://addons/beatsmr/shaders/parcel_renderer/main_image.glslinc"

void main() {
	vec4 fragColor = vec4(0.0);
	mainImage(fragColor, gl_FragCoord.xy);
	out_color = fragColor;
}
