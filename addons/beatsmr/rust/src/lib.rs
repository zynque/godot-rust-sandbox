use godot::prelude::*;

mod sort_tester;

struct BeatsmrExtension;

#[gdextension]
unsafe impl ExtensionLibrary for BeatsmrExtension {}
