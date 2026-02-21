#[compute]
#version 450

// Bindless-style storage buffers - access any index without binding limits
// Perfect for massive procedural scenes with millions of instances
layout(set = 0, binding = 0, std430) restrict buffer InstanceData {
    vec4 positions[];  // xyz = position, w = scale
} instance_data;

layout(set = 0, binding = 1, std430) restrict buffer ColorData {
    vec4 colors[];     // rgba colors
} color_data;

// 64 threads per workgroup for optimal GPU occupancy
layout(local_size_x = 64, local_size_y = 1, local_size_z = 1) in;

// Simple hash function for procedural generation
float hash(float n) {
    return fract(sin(n) * 43758.5453123);
}

float hash2(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
}

void main() {
    uint idx = gl_GlobalInvocationID.x;
    
    // Procedurally generate positions in spiral pattern
    // This is perfect for foliage - adjust parameters for different distributions
    float angle = float(idx) * 0.1;
    float radius = 5.0 + hash(float(idx)) * 20.0;
    float height = hash(float(idx) * 1.5) * 10.0;
    
    // For foliage, you'd want more natural clustering:
    // - Poisson disk sampling for even distribution
    // - Perlin noise for organic patterns
    // - Biome-based density variations
    
    instance_data.positions[idx] = vec4(
        cos(angle) * radius,
        height,
        sin(angle) * radius,
        0.5 + hash(float(idx) * 2.0) * 0.5  // scale variation
    );
    
    // Procedural colors - for foliage, use texture indices instead
    color_data.colors[idx] = vec4(
        hash(float(idx)),
        hash(float(idx) + 100.0),
        hash(float(idx) + 200.0),
        1.0
    );
    
    // Advanced: Add LOD info, wind animation data, etc. in unused channels
    // instance_data.positions[idx].w could encode LOD level
    // color_data.colors[idx].a could encode texture array index
}
