use godot::prelude::*;

mod parcel_renderer_node;
mod sort_tester;
// pub use sort_tester::SortTester;

struct BeatsmrExtension;

#[gdextension]
unsafe impl ExtensionLibrary for BeatsmrExtension {}
