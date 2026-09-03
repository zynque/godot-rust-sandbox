use godot::classes::{Node2D, RenderingDevice};
use godot::prelude::*;

mod lifecycle;
mod pipeline;
mod preview;

const PREVIEW_NAME: &str = "ParcelRendererPreview";
const SHADER_PATH: &str = "res://addons/beatsmr/shaders/parcel_renderer/parcel_renderer.glsl";

#[derive(GodotClass)]
#[class(tool, base=Node2D)]
pub struct ParcelRendererNode {
    base: Base<Node2D>,
    rd: Option<Gd<RenderingDevice>>,
    texture_rid: Rid,
    time_buffer_rid: Rid,
    shader_rid: Rid,
    pipeline_rid: Rid,
    uniform_set_rid: Rid,
    width: u32,
    height: u32,
    time_seconds: f32,
    #[export]
    animate: bool,
    logged_waiting_for_size: bool,
    editor_selected: bool,
    editor_selection_known: bool,
}

#[godot_api]
impl INode2D for ParcelRendererNode {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            base,
            rd: None,
            texture_rid: Rid::Invalid,
            time_buffer_rid: Rid::Invalid,
            shader_rid: Rid::Invalid,
            pipeline_rid: Rid::Invalid,
            uniform_set_rid: Rid::Invalid,
            width: 0,
            height: 0,
            time_seconds: 0.0,
            animate: true,
            logged_waiting_for_size: false,
            editor_selected: false,
            editor_selection_known: false,
        }
    }

    fn ready(&mut self) {
        self.on_ready();
    }

    fn process(&mut self, delta: f64) {
        self.on_process(delta);
    }

    fn exit_tree(&mut self) {
        self.on_exit_tree();
    }
}

#[godot_api]
impl ParcelRendererNode {
    #[func]
    fn set_editor_selected(&mut self, selected: bool) {
        self.editor_selected = selected;
        self.editor_selection_known = true;
    }
}
