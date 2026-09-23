use godot::prelude::*;

mod intersect_parcels_tester;
mod parcel_renderer_node;
mod shader_test_common;
mod sort_intervals_tester;

struct BeatsmrExtension;

#[gdextension]
unsafe impl ExtensionLibrary for BeatsmrExtension {}
