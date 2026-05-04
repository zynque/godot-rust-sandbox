use godot::prelude::*;
use godot::classes::{
    Camera3D,
    Engine,
    RenderingServer,
    RenderingDevice,
    TextureRect,
};

mod gpu_splat_data;

mod texture;
mod pipeline;
mod uniform;
mod dispatch;
mod display;

use self::dispatch::dispatch_compute_raw;
use self::uniform::{camera_uniform_bytes, update_camera_buffer_raw};

impl SplatViewer {
    fn in_editor(&self) -> bool {
        Engine::singleton().is_editor_hint()
    }

    fn camera_data_for_frame(&mut self) -> PackedByteArray {
        if let Some(vp) = self.base().get_viewport() {
            if let Some(camera) = vp.get_camera_3d() {
                return camera_uniform_bytes(
                    camera.get_camera_transform(),
                    camera.get_camera_projection(),
                );
            }
        }

        if let Some(camera) = self
            .base()
            .try_get_node_as::<Camera3D>("../Camera3D")
            .or_else(|| self.base().try_get_node_as::<Camera3D>("Camera3D"))
        {
            return camera_uniform_bytes(
                camera.get_camera_transform(),
                camera.get_camera_projection(),
            );
        }

        if self.in_editor() && !self.warned_editor_no_camera {
            godot_warn!(
                "SplatViewer(editor): no Camera3D found from viewport or paths ../Camera3D and Camera3D; using identity matrices."
            );
            self.warned_editor_no_camera = true;
        }

        camera_uniform_bytes(Transform3D::IDENTITY, Projection::IDENTITY)
    }
}

#[derive(GodotClass)]
#[class(tool, base=Node)]
struct SplatViewer {
    base: Base<Node>,

    rd: Option<Gd<RenderingDevice>>,
    texture_rid: Rid,
    splat_buffer_rid: Rid,
    camera_buffer_rid: Rid,
    uniform_set_rid: Rid,
    shader_rid: Rid,
    pipeline_rid: Rid,

    width: u32,
    height: u32,

    warned_editor_no_camera: bool,
    warned_editor_no_rd: bool,
    warned_editor_invalid_pipeline: bool,
    warned_editor_missing_preview: bool,
}

#[godot_api]
impl INode for SplatViewer {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            rd: None,
            texture_rid: Rid::Invalid,
            splat_buffer_rid: Rid::Invalid,
            camera_buffer_rid: Rid::Invalid,
            uniform_set_rid: Rid::Invalid,
            shader_rid: Rid::Invalid,
            pipeline_rid: Rid::Invalid,
            width: 512,
            height: 512,
            warned_editor_no_camera: false,
            warned_editor_no_rd: false,
            warned_editor_invalid_pipeline: false,
            warned_editor_missing_preview: false,
        }
    }

    fn ready(&mut self) {
        godot_print!("SplatViewer ready(); initializing compute resources.");
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
            self.create_camera_buffer(&mut rd);
            self.create_uniform_set(&mut rd);

            if self.in_editor() && !self.warned_editor_invalid_pipeline {
                if self.texture_rid == Rid::Invalid
                    || self.shader_rid == Rid::Invalid
                    || self.pipeline_rid == Rid::Invalid
                    || self.uniform_set_rid == Rid::Invalid
                {
                    godot_warn!(
                        "SplatViewer(editor): compute resources invalid (texture/shader/pipeline/uniform set). Check shader import and RenderingDevice availability."
                    );
                    self.warned_editor_invalid_pipeline = true;
                }
            }

            let camera_data = self.camera_data_for_frame();
            update_camera_buffer_raw(&mut rd, self.camera_buffer_rid, &camera_data);

            self.dispatch_compute(&mut rd);
            self.display_result();

            if self.in_editor()
                && !self.warned_editor_missing_preview
                && self
                    .base()
                    .try_get_node_as::<TextureRect>("../SplatPreview")
                    .or_else(|| self.base().try_get_node_as::<TextureRect>("SplatPreview"))
                    .is_none()
            {
                godot_warn!(
                    "SplatViewer(editor): preview TextureRect child SplatPreview was not created; output cannot be displayed in editor viewport."
                );
                self.warned_editor_missing_preview = true;
            }

            self.rd = Some(rd);
        } else {
            if self.in_editor() && !self.warned_editor_no_rd {
                godot_warn!(
                    "SplatViewer(editor): failed to get main RenderingDevice in ready(); zero-copy display unavailable in editor."
                );
                self.warned_editor_no_rd = true;
            } else {
                godot_warn!("SplatViewer: failed to get main RenderingDevice in ready(); zero-copy display unavailable.");
            }
        }
    }

    fn process(&mut self, _delta: f64) {
        if self.rd.is_none() {
            return;
        }

        let pipeline_rid = self.pipeline_rid;
        let uniform_set_rid = self.uniform_set_rid;
        let camera_buffer_rid = self.camera_buffer_rid;
        let width = self.width;
        let height = self.height;

        let camera_data = self.camera_data_for_frame();

        let callable = Callable::from_local_fn("splat_render_thread_dispatch", move |_| {
            if let Some(mut rd) = RenderingServer::singleton().get_rendering_device() {
                update_camera_buffer_raw(&mut rd, camera_buffer_rid, &camera_data);
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

        if let Some(mut rect) = self
            .base()
            .try_get_node_as::<TextureRect>("../SplatPreview")
            .or_else(|| self.base().try_get_node_as::<TextureRect>("SplatPreview"))
        {
            rect.queue_redraw();
        } else if self.in_editor() && !self.warned_editor_missing_preview {
            godot_warn!(
                "SplatViewer(editor): missing SplatPreview TextureRect during process(); nothing will be visible in the editor viewport."
            );
            self.warned_editor_missing_preview = true;
        }
    }
}
