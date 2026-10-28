use godot::prelude::*;

mod example_scene;

struct SandboxExtension;

#[gdextension]
unsafe impl ExtensionLibrary for SandboxExtension {}
