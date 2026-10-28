use godot::classes::{Node2D, RenderingDevice};
use godot::prelude::*;

mod lifecycle;
mod pipeline;
mod preview;

use pipeline::{MAX_PARCELS, PARCEL_FLOATS_PER_INSTANCE};

const PREVIEW_NAME: &str = "ParcelRendererPreview";
const SHADER_PATH: &str = "res://addons/beatsmr/shaders/parcel_renderer/parcel_renderer.glsl";

#[derive(GodotClass)]
#[class(tool, base=Node2D)]
pub struct ParcelRendererNode {
    base: Base<Node2D>,
    rd: Option<Gd<RenderingDevice>>,
    texture_rid: Rid,
    time_buffer_rid: Rid,
    camera_buffer_rid: Rid,
    parcel_buffer_rid: Rid,
    shader_rid: Rid,
    pipeline_rid: Rid,
    uniform_set_rid: Rid,
    width: u32,
    height: u32,
    time_seconds: f32,
    /// Packed parcel records supplied by `set_parcels`, uploaded when dirty.
    parcel_data: Vec<f32>,
    parcel_data_dirty: bool,
    #[export]
    animate: bool,
    /// Fraction of the project viewport the compute shader renders at. The
    /// preview is scaled back up, so values below 1 trade sharpness for speed.
    #[export]
    render_scale: f32,
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
            camera_buffer_rid: Rid::Invalid,
            parcel_buffer_rid: Rid::Invalid,
            shader_rid: Rid::Invalid,
            pipeline_rid: Rid::Invalid,
            uniform_set_rid: Rid::Invalid,
            width: 0,
            height: 0,
            time_seconds: 0.0,
            parcel_data: Vec::new(),
            parcel_data_dirty: true,
            animate: true,
            render_scale: 1.0,
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
    /// Replaces the rendered parcels with packed `Parcel` records of
    /// `PARCEL_FLOATS_PER_INSTANCE` floats each, as produced by the growth
    /// addon. Invalid input is rejected and the previous parcels are kept.
    #[func]
    fn set_parcels(&mut self, parcels: PackedFloat32Array) {
        let values = parcels.as_slice();
        if values.len() % PARCEL_FLOATS_PER_INSTANCE != 0 {
            godot_error!(
                "ParcelRendererNode: set_parcels expects packed records of {} floats, got {} floats.",
                PARCEL_FLOATS_PER_INSTANCE,
                values.len(),
            );
            return;
        }

        let count = values.len() / PARCEL_FLOATS_PER_INSTANCE;
        if count > MAX_PARCELS {
            godot_error!(
                "ParcelRendererNode: set_parcels received {} parcels but the buffer holds at most {}.",
                count,
                MAX_PARCELS,
            );
            return;
        }

        let previous_count = self.parcel_data.len() / PARCEL_FLOATS_PER_INSTANCE;
        self.parcel_data.clear();
        self.parcel_data.extend_from_slice(values);
        self.parcel_data_dirty = true;
        if count != previous_count {
            godot_print!("ParcelRendererNode: received {} parcels.", count);
        }
    }

    #[func]
    fn set_editor_selected(&mut self, selected: bool) {
        self.editor_selected = selected;
        self.editor_selection_known = true;
    }
}
