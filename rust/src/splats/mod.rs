use godot::prelude::*;
use godot::classes::{
    RenderingServer,
    RenderingDevice,
};

mod gpu_splat_data;

mod texture;
mod pipeline;
mod uniform;
mod dispatch;
mod display;

use self::dispatch::dispatch_compute_raw;

#[derive(GodotClass)]
#[class(base=Node)]
struct SplatViewer {
    base: Base<Node>,

    rd: Option<Gd<RenderingDevice>>,
    texture_rid: Rid,
    splat_buffer_rid: Rid,
    uniform_set_rid: Rid,
    shader_rid: Rid,
    pipeline_rid: Rid,

    width: u32,
    height: u32,
}

#[godot_api]
impl INode for SplatViewer {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            rd: None,
            texture_rid: Rid::Invalid,
            splat_buffer_rid: Rid::Invalid,
            uniform_set_rid: Rid::Invalid,
            shader_rid: Rid::Invalid,
            pipeline_rid: Rid::Invalid,
            width: 512,
            height: 512,
        }
    }

    fn ready(&mut self) {
        self.rd = RenderingServer::singleton().get_rendering_device();
        self.base_mut().set_process(true);

        let visible_rect = self.base().get_viewport()
          .map(|vp| vp.get_visible_rect().size)
          .unwrap_or(Vector2::new(512.0, 512.0));

        self.width = visible_rect.x as u32;
        self.height = visible_rect.y as u32;

        if let Some(mut rd) = self.rd.take() {
            self.create_texture(&mut rd);
            self.create_pipeline(&mut rd);
            self.create_splat_buffer(&mut rd);
            self.create_uniform_set(&mut rd);
            self.dispatch_compute(&mut rd);
            self.display_result();
            self.rd = Some(rd);
        } else {
            godot_warn!("SplatViewer: failed to get main RenderingDevice in ready(); zero-copy display unavailable.");
        }
    }

    fn process(&mut self, _delta: f64) {
        if self.rd.is_none() {
            return;
        }

        let pipeline_rid = self.pipeline_rid;
        let uniform_set_rid = self.uniform_set_rid;
        let width = self.width;
        let height = self.height;

        let callable = Callable::from_local_fn("splat_render_thread_dispatch", move |_| {
            if let Some(mut rd) = RenderingServer::singleton().get_rendering_device() {
                dispatch_compute_raw(
                    &mut rd,
                    pipeline_rid,
                    uniform_set_rid,
                    width,
                    height,
                );
            }

            Ok(Variant::nil())
        });

        let mut rs = RenderingServer::singleton();
        rs.call_on_render_thread(&callable);
    }
}
