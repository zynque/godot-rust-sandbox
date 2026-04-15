#[compute]
#version 450

layout(local_size_x = 8, local_size_y = 8) in;

layout(rgba32f, binding = 0) uniform image2D output_image;

void main() {
    ivec2 pixel = ivec2(gl_GlobalInvocationID.xy);
    ivec2 size = imageSize(output_image);

    vec2 uv = vec2(pixel) / vec2(size);

    vec3 color = vec3(uv, 0.5);

    imageStore(output_image, pixel, vec4(color, 1.0));
}
