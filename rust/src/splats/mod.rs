use godot::prelude::*;
use godot::classes::{
    RenderingServer,
    RenderingDevice,
};

mod texture;
mod pipeline;
mod uniform;
mod dispatch;
mod display;

#[derive(GodotClass)]
#[class(base=Node)]
struct SplatViewer {
    base: Base<Node>,

    rd: Option<Gd<RenderingDevice>>,
    texture_rid: Rid,
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
            uniform_set_rid: Rid::Invalid,
            shader_rid: Rid::Invalid,
            pipeline_rid: Rid::Invalid,
            width: 512,
            height: 512,
        }
    }

    fn ready(&mut self) {
        self.rd = RenderingServer::singleton().create_local_rendering_device();

        let visible_rect = self.base().get_viewport()
          .map(|vp| vp.get_visible_rect().size)
          .unwrap_or(Vector2::new(512.0, 512.0));

        self.width = visible_rect.x as u32;
        self.height = visible_rect.y as u32;

        if let Some(mut rd) = self.rd.take() {
            self.create_texture(&mut rd);
            self.create_pipeline(&mut rd);
            self.create_uniform_set(&mut rd);
            self.dispatch_compute(&mut rd);
            self.display_result();
            self.rd = Some(rd);
        } else {
            godot_warn!("SplatViewer: failed to create local RenderingDevice in ready(); compute output disabled.");
        }
    }
}
