#[compute]
#version 450

#define RADIX_SORT_MAX_ITEMS 128u
#include "res://radix_sort.glslinc"

layout(local_size_x = 8, local_size_y = 8) in;

layout(rgba32f, binding = 0) uniform image2D output_image;

struct SplatData {
    vec4 position_density;
    vec4 cov0_pad;
    vec4 cov1_pad;
    vec4 cov2_pad;
    vec4 color_pad;
};

layout(std430, binding = 1) readonly buffer SplatBuffer {
    SplatData items[];
} splats;

layout(std140, binding = 2) uniform CameraUniform {
    mat4 view;
    mat4 projection;
} camera;

void main() {
    ivec2 pixel = ivec2(gl_GlobalInvocationID.xy);
    ivec2 size = imageSize(output_image);

    if (pixel.x >= size.x || pixel.y >= size.y) {
        return;
    }

    vec2 uv = vec2(pixel) / vec2(size);

    vec3 color = vec3(0.02, 0.02, 0.03);

    uint sort_keys[RADIX_SORT_MAX_ITEMS];
    uint sort_indices[RADIX_SORT_MAX_ITEMS];

    uint splat_count = min(splats.items.length(), RADIX_SORT_MAX_ITEMS);
    for (uint i = 0u; i < splat_count; i++) {
        SplatData splat = splats.items[i];

        vec4 world_pos = vec4(splat.position_density.xyz, 1.0);
        vec4 view_pos = camera.view * world_pos;

        float depth = -view_pos.z;
        uint depth_key = floatBitsToUint(max(depth, 0.0));

        // Ascending radix sort + reverse traversal gives back-to-front order.
        sort_keys[i] = depth_key;
        sort_indices[i] = i;
    }

    radix_sort_u32_indices(sort_keys, sort_indices, splat_count);

    for (uint order = 0u; order < splat_count; order++) {
        uint sorted_i = splat_count - 1u - order;
        SplatData splat = splats.items[sort_indices[sorted_i]];
        float sigma2 = max(splat.cov0_pad.x, 0.00001);

        vec4 world_pos = vec4(splat.position_density.xyz, 1.0);
        vec4 clip_pos = camera.projection * camera.view * world_pos;
        if (clip_pos.w <= 0.0001) {
            continue;
        }

        vec2 ndc = clip_pos.xy / clip_pos.w;
        vec2 center = vec2(0.5 + 0.5 * ndc.x, 0.5 - 0.5 * ndc.y);
        vec2 d = uv - center;
        float dist2 = dot(d, d);
        float weight = splat.position_density.w * exp(-dist2 / (2.0 * sigma2));
        float alpha = clamp(1.0 - exp(-weight), 0.0, 1.0);

        // Source-over compositing in back-to-front order.
        color = splat.color_pad.xyz * alpha + color * (1.0 - alpha);
    }

    color = clamp(color, vec3(0.0), vec3(1.0));

    imageStore(output_image, pixel, vec4(color, 1.0));
}
