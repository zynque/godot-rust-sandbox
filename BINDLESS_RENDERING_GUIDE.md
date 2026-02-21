# Bindless Rendering in Godot with Rust

This guide shows how to use bindless rendering techniques in Godot for massive procedural scenes with complex foliage.

## What is Bindless Rendering?

Bindless rendering allows shaders to access large arrays of data (buffers, textures) through indices rather than binding each resource individually. This enables:

- **Millions of instances** rendered efficiently
- **No binding limits** - traditional APIs limit active textures/buffers
- **GPU-driven rendering** - compute shaders generate/cull geometry
- **Procedural generation** - create complex scenes on the GPU

## Requirements

1. **Godot 4.4** with Forward+ or Mobile renderer (NOT Compatibility)
2. **Vulkan support** - RenderingDevice requires modern graphics APIs
3. **godot-rust 0.3.5+**

### Enable Forward+ Renderer

In Godot: `Project Settings > Rendering > Renderer > Rendering Method`
- Set to **"Forward Plus"** (best quality) or **"Mobile"**
- Compatibility renderer does NOT support RenderingDevice

## File Structure

```
rust/
  src/
    bindless_rendering.rs   # Rust implementation
bindless_compute.gdshader   # Compute shader
```

## Basic Usage

### 1. Add to Scene

In Godot editor:
1. Add a Node3D to your scene
2. Attach script: `BindlessRenderer` (from Rust)
3. Set instance count in inspector

### 2. Load Shader (TODO in code)

```rust
// In ready() after creating buffers:
let shader_path = "res://bindless_compute.gdshader";
let shader_file = load::<RDShaderFile>(shader_path);
// Compile and create pipeline...
```

### 3. Dispatch Compute

```rust
// In process():
self.dispatch_compute();
```

## Shader Structure

### Storage Buffers

```glsl
// Bindless access - index into huge arrays
layout(set = 0, binding = 0, std430) buffer InstanceData {
    vec4 positions[];  
} instance_data;

// Access by compute thread ID
uint idx = gl_GlobalInvocationID.x;
instance_data.positions[idx] = vec4(...);
```

### Workgroups

```glsl
layout(local_size_x = 64) in;
```

- 64/128/256 threads per workgroup common
- Dispatch: `workgroups = (instance_count + 63) / 64`

## For Large Foliage Scenes

### Techniques to Implement

1. **Frustum Culling**
   ```glsl
   // Pass camera frustum planes as uniform
   bool visible = isInFrustum(position, frustum);
   if (visible) {
       // Write to visible instances buffer
   }
   ```

2. **LOD Selection**
   ```glsl
   float distance = length(camera_pos - position);
   uint lod = clamp(uint(distance / 10.0), 0, 3);
   // Store LOD in instance data
   ```

3. **Indirect Drawing**
   - Compute shader writes to indirect buffer
   - GPU decides draw counts
   - Zero CPU overhead

4. **Texture Arrays** (True Bindless)
   ```glsl
   layout(set = 1, binding = 0) uniform sampler2D textures[];
   uint tex_idx = get_texture_index(instance_id);
   vec4 color = texture(textures[tex_idx], uv);
   ```

5. **Procedural Distribution**
   - Poisson disk sampling for even spacing
   - Perlin/Simplex noise for natural clustering
   - Biome data for density variations

## Performance Tips

### Compute Shader

- **Shared memory**: Use `shared` for workgroup-local data
- **Memory barriers**: Add `memoryBarrierBuffer()` when needed
- **Coalesced access**: Sequential threads access sequential memory

### Buffer Size

```rust
// Align to 16 bytes (vec4)
let buffer_size = instance_count * 16; // bytes

// For transform matrices, use mat4 = 64 bytes
let transform_size = instance_count * 64;
```

### Async Rendering

```rust
// Don't sync every frame!
rd.submit();
// rd.sync(); // Only for debugging

// Use fences/semaphores for multi-frame
```

## Rendering the Instances

### Option 1: MultiMeshInstance3D

```rust
// Read back compute results
let data = rd.buffer_get_data(instance_buffer);

// Update MultiMesh
let mut multimesh = get_node::<MultiMeshInstance3D>("Instances");
for i in 0..count {
    let transform = read_transform(&data, i);
    multimesh.set_instance_transform(i, transform);
}
```

### Option 2: Custom Vertex Shader (True Bindless)

```glsl
#[vertex]
// Read instance data directly in shader
layout(set = 0, binding = 0, std430) readonly buffer InstanceData {
    vec4 positions[];
} instance_data;

void vertex() {
    vec4 inst = instance_data.positions[gl_InstanceIndex];
    VERTEX = VERTEX * inst.w + inst.xyz; // scale and translate
}
```

## Advanced: Indirect Drawing

```rust
// Create indirect buffer
let indirect_buffer = rd.indirect_buffer_create(...);

// Compute shader writes draw commands
struct DrawIndirectCommand {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

// Draw with zero CPU involvement
rd.draw_list_draw_indirect(draw_list, indirect_buffer, ...);
```

## Example: Million-Instance Foliage

```rust
pub struct MassiveFoliage {
    instance_count: u32,  // 1,000,000
    
    // Buffers
    position_buffer: Rid,
    transform_buffer: Rid,
    lod_buffer: Rid,
    visible_buffer: Rid,  // Culling results
    indirect_buffer: Rid, // Draw commands
    
    // Pipelines
    generation_pipeline: Rid,  // Procedural placement
    culling_pipeline: Rid,     // Frustum/occlusion
    lod_pipeline: Rid,         // LOD selection
}

impl MassiveFoliage {
    fn update(&mut self, camera: Transform3D) {
        // 1. Generate/update positions (once or per frame)
        self.dispatch(generation_pipeline, workgroups);
        
        // 2. Cull invisible instances
        self.set_camera_uniform(camera);
        self.dispatch(culling_pipeline, workgroups);
        
        // 3. Select LODs based on distance
        self.dispatch(lod_pipeline, workgroups);
        
        // 4. Indirect draw (GPU-driven)
        rd.draw_list_draw_indirect(...);
    }
}
```

## Common Issues

### "RenderingDevice not available"
- Check renderer is Forward+ or Mobile
- Restart Godot editor after changing

### Shader compilation fails
- Use `#[compute]` annotation in .gdshader
- Check GLSL version (450 for Vulkan)
- Validate with `glslangValidator` externally

### Buffer overflow
- Ensure buffer size matches instance count
- Check alignment (vec4 = 16 bytes)
- Use `restrict` keyword to help compiler

## Next Steps

1. Load shader from .gdshader file
2. Implement compute dispatch in `process()`
3. Add camera frustum culling
4. Integrate with MultiMesh or custom rendering
5. Add texture arrays for material variation
6. Implement LOD system
7. Profile with RenderDoc/NSight Graphics

## Resources

- [Godot RenderingDevice docs](https://docs.godotengine.org/en/stable/classes/class_renderingdevice.html)
- [godot-rust book](https://godot-rust.github.io/)
- [GPU-Driven Rendering (YouTube)](https://www.youtube.com/results?search_query=gpu+driven+rendering)
- [Indirect Drawing in Vulkan](https://www.khronos.org/opengl/wiki/Vertex_Rendering#Indirect_rendering)
