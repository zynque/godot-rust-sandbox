use godot::classes::{Engine, RenderingServer};
use godot::prelude::*;

use super::ParcelRendererNode;

impl ParcelRendererNode {
    fn resolve_render_size(&self) -> Vector2 {
        let rect_size = self.base().get_viewport_rect().size;
        let viewport_size = self
            .base()
            .get_viewport()
            .map(|vp| vp.get_visible_rect().size)
            .unwrap_or(Vector2::ZERO);

        if rect_size.x * rect_size.y >= viewport_size.x * viewport_size.y {
            rect_size
        } else {
            viewport_size
        }
    }

    fn refresh_render_size_if_needed(&mut self) {
        let size = self.resolve_render_size();
        let new_w = size.x as u32;
        let new_h = size.y as u32;

        if new_w == self.width && new_h == self.height {
            return;
        }

        self.width = new_w;
        self.height = new_h;

        if let Some(mut rd) = self.rd.take() {
            self.recreate_resources(&mut rd);
            self.attach_preview_texture();
            self.rd = Some(rd);
        }
    }

    pub(super) fn on_ready(&mut self) {
        self.base_mut().set_process(true);

        let size = self.resolve_render_size();
        self.width = size.x as u32;
        self.height = size.y as u32;

        self.rd = RenderingServer::singleton().get_rendering_device();
        let Some(mut rd) = self.rd.take() else {
            godot_warn!("ParcelRendererNode: failed to get RenderingDevice.");
            return;
        };

        self.create_texture(&mut rd);
        self.create_pipeline_and_uniforms(&mut rd);
        self.attach_preview_texture();

        self.rd = Some(rd);
    }

    pub(super) fn on_process(&mut self, _delta: f64) {
        let in_editor = Engine::singleton().is_editor_hint();
        let show_preview = !in_editor || !self.editor_selection_known || self.editor_selected;
        self.set_preview_visible(show_preview);
        if !show_preview {
            return;
        }

        self.refresh_render_size_if_needed();

        if self.pipeline_rid == Rid::Invalid || self.uniform_set_rid == Rid::Invalid {
            return;
        }

        if let Some(mut rd) = self.rd.take() {
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
        if let Some(mut rd) = self.rd.take() {
            self.destroy_resources(&mut rd);
        }
    }
}
