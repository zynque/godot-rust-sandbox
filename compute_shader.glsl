#[compute]
#version 450

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

    uint splat_count = splats.items.length();
    for (uint i = 0u; i < splat_count; i++) {
        SplatData splat = splats.items[i];
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

        color += splat.color_pad.xyz * weight;
    }

    color = clamp(color, vec3(0.0), vec3(1.0));

    imageStore(output_image, pixel, vec4(color, 1.0));
}
