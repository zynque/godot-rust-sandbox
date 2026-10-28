Growth
Generates parcels for the parcel renderer in the beatsmr addon. A parcel is a gaussian ellipsoid of matter with a color; collections of them form the rendered scene.

This is a Rust library addon, not a Godot plugin: it is compiled into whichever crate depends on it, such as the project's root `rust` crate.

- `rust/src/parcels.rs` - parcel records and the shape-to-covariance math, packed into the GPU layout the renderer expects.
- `rust/src/tree.rs` - a small branching tree generator.
- `rust/src/example_scene.rs` - the coordinate axes, the example parcels, and the tree combined into a demo scene.

The root `rust/src/example_scene.rs` builds `example_scene_parcels_at(time)`, packs each parcel with `Parcel::as_f32s()`, and hands the data to `ParcelRendererNode.set_parcels()` every frame while the renderer's `animate` flag is on.
