use godot::prelude::*;

pub mod physics;
pub mod lsystem;
pub mod input_state;
pub mod lsystem_viewer;
pub mod splats;
pub mod segments;
pub mod beatsmr;

struct GodotRustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for GodotRustExtension {}
