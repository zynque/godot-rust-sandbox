use godot::classes::{Engine, ProjectSettings, RenderingServer};
use godot::prelude::*;

use super::ParcelRendererNode;

impl ParcelRendererNode {
    fn ensure_render_device(&mut self) -> bool {
        if self.rd.is_some() {
            return true;
        }

        self.rd = RenderingServer::singleton().get_rendering_device();
        if self.rd.is_none() {
            godot_warn!("ParcelRendererNode: RenderingDevice unavailable in current editor state.");
            return false;
        }

        godot_print!("ParcelRendererNode: reacquired RenderingDevice.");
        true
    }

    fn resolve_render_size(&self) -> Vector2 {
        let settings = ProjectSettings::singleton();

        let width_v = settings.get_setting("display/window/size/viewport_width");
        let height_v = settings.get_setting("display/window/size/viewport_height");

        let Ok(width) = width_v.try_to::<i64>() else {
            godot_error!(
                "ParcelRendererNode: project setting display/window/size/viewport_width is not an integer."
            );
            return Vector2::ZERO;
        };

        let Ok(height) = height_v.try_to::<i64>() else {
            godot_error!(
                "ParcelRendererNode: project setting display/window/size/viewport_height is not an integer."
            );
            return Vector2::ZERO;
        };

        if width <= 0 || height <= 0 {
            godot_error!(
                "ParcelRendererNode: invalid project viewport size {}x{} from ProjectSettings.",
                width,
                height,
            );
            return Vector2::ZERO;
        }

        Vector2::new(width as f32, height as f32)
    }

    fn resources_ready(&self) -> bool {
        self.texture_rid != Rid::Invalid
            && self.time_buffer_rid != Rid::Invalid
            && self.shader_rid != Rid::Invalid
            && self.pipeline_rid != Rid::Invalid
            && self.uniform_set_rid != Rid::Invalid
    }

    fn sync_size_and_resources(&mut self) {
        if !self.ensure_render_device() {
            return;
        }

        let size = self.resolve_render_size();
        let new_w = size.x as u32;
        let new_h = size.y as u32;

        if new_w <= 2 || new_h <= 2 {
            if !self.logged_waiting_for_size {
                godot_print!(
                    "ParcelRendererNode waiting for valid render size from ProjectSettings; current={}x{}.",
                    new_w,
                    new_h,
                );
                self.logged_waiting_for_size = true;
            }
            return;
        }

        if self.logged_waiting_for_size {
            godot_print!(
                "ParcelRendererNode received valid project viewport size={}x{}.",
                new_w,
                new_h,
            );
            self.logged_waiting_for_size = false;
        }

        let resized = new_w != self.width || new_h != self.height;
        if resized {
            self.width = new_w;
            self.height = new_h;
            godot_print!(
                "ParcelRendererNode using render target {}x{}.",
                self.width,
                self.height,
            );
        }

        if let Some(mut rd) = self.rd.take() {
            if resized {
                self.remove_preview_node();
                self.recreate_resources(&mut rd);
                self.attach_preview_texture();
            } else if !self.resources_ready() {
                self.create_texture(&mut rd);
                self.create_pipeline_and_uniforms(&mut rd);
                self.attach_preview_texture();
            } else if !self.has_preview_node() {
                self.attach_preview_texture();
            }

            self.rd = Some(rd);
        }
    }

    pub(super) fn on_ready(&mut self) {
        self.base_mut().set_process(true);

        self.sync_size_and_resources();
    }

    pub(super) fn on_process(&mut self, delta: f64) {
        let in_editor = Engine::singleton().is_editor_hint();
        let show_preview = !in_editor || !self.editor_selection_known || self.editor_selected;
        self.set_preview_visible(show_preview);
        if !show_preview {
            return;
        }

        self.sync_size_and_resources();

        if self.pipeline_rid == Rid::Invalid || self.uniform_set_rid == Rid::Invalid {
            return;
        }

        if self.animate {
            self.time_seconds += delta as f32;
        }

        if let Some(mut rd) = self.rd.take() {
            if self.time_buffer_rid != Rid::Invalid {
                let time_data = [self.time_seconds, 0.0f32, 0.0f32, 0.0f32];
                let mut bytes = Vec::with_capacity(16);
                for f in time_data {
                    bytes.extend_from_slice(&f.to_le_bytes());
                }
                let packed = PackedByteArray::from(bytes.as_slice());
                let _ = rd.buffer_update(self.time_buffer_rid, 0, 16, &packed);
            }

            let list = rd.compute_list_begin();
            rd.compute_list_bind_compute_pipeline(list, self.pipeline_rid);
            rd.compute_list_bind_uniform_set(list, self.uniform_set_rid, 0);
            rd.compute_list_dispatch(list, self.width.div_ceil(8), self.height.div_ceil(8), 1);
            rd.compute_list_end();
            self.rd = Some(rd);
        }

        self.base_mut().queue_redraw();
    }

    pub(super) fn on_exit_tree(&mut self) {
        self.remove_preview_node();

        if let Some(mut rd) = self.rd.take() {
            self.destroy_resources(&mut rd);
        }
    }
}
