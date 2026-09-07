use godot::prelude::*;

mod insertion_sort_by_float_tester;
mod parcel_renderer_node;
mod shader_test_common;
mod sort_tester;

struct BeatsmrExtension;

#[gdextension]
unsafe impl ExtensionLibrary for BeatsmrExtension {}
