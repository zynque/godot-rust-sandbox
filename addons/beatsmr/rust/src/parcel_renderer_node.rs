use godot::classes::rendering_device::{DataFormat, TextureUsageBits, UniformType};
use godot::classes::{
    Engine, Node2D, RdShaderFile, RdTextureFormat, RdTextureView, RdUniform, RenderingDevice,
    RenderingServer, Sprite2D, Texture2Drd,
};
use godot::prelude::*;

const PREVIEW_NAME: &str = "ParcelRendererPreview";
const SHADER_PATH: &str = "res://addons/beatsmr/shaders/parcel_renderer/parcel_renderer.glsl";

#[derive(GodotClass)]
#[class(tool, base=Node2D)]
pub struct ParcelRendererNode {
    base: Base<Node2D>,
    rd: Option<Gd<RenderingDevice>>,
    texture_rid: Rid,
    shader_rid: Rid,
    pipeline_rid: Rid,
    uniform_set_rid: Rid,
    width: u32,
    height: u32,
    editor_selected: bool,
}

#[godot_api]
impl INode2D for ParcelRendererNode {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            base,
            rd: None,
            texture_rid: Rid::Invalid,
            shader_rid: Rid::Invalid,
            pipeline_rid: Rid::Invalid,
            uniform_set_rid: Rid::Invalid,
            width: 512,
            height: 512,
            editor_selected: false,
        }
    }

    fn ready(&mut self) {
        self.base_mut().set_process(true);

        if let Some(vp) = self.base().get_viewport() {
            let size = vp.get_visible_rect().size;
            self.width = size.x.max(1.0) as u32;
            self.height = size.y.max(1.0) as u32;
        }

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

    fn process(&mut self, _delta: f64) {
        let show_preview = !Engine::singleton().is_editor_hint() || self.editor_selected;
        self.set_preview_visible(show_preview);
        if !show_preview {
            return;
        }

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

    fn exit_tree(&mut self) {
        if let Some(mut rd) = self.rd.take() {
            if self.uniform_set_rid != Rid::Invalid {
                rd.free_rid(self.uniform_set_rid);
            }
            if self.pipeline_rid != Rid::Invalid {
                rd.free_rid(self.pipeline_rid);
            }
            if self.shader_rid != Rid::Invalid {
                rd.free_rid(self.shader_rid);
            }
            if self.texture_rid != Rid::Invalid {
                rd.free_rid(self.texture_rid);
            }
        }
    }
}

#[godot_api]
impl ParcelRendererNode {
    #[func]
    fn set_editor_selected(&mut self, selected: bool) {
        self.editor_selected = selected;
    }

    fn create_texture(&mut self, rd: &mut Gd<RenderingDevice>) {
        let usage = TextureUsageBits::STORAGE_BIT | TextureUsageBits::SAMPLING_BIT;
        let candidate_formats = [
            DataFormat::R32G32B32A32_SFLOAT,
            DataFormat::R16G16B16A16_SFLOAT,
            DataFormat::R8G8B8A8_UNORM,
        ];

        for data_format in candidate_formats {
            if !rd.texture_is_format_supported_for_usage(data_format, usage) {
                continue;
            }

            let mut format = RdTextureFormat::new_gd();
            format.set_width(self.width);
            format.set_height(self.height);
            format.set_format(data_format);
            format.set_usage_bits(usage);

            let view = RdTextureView::new_gd();
            let texture_rid = rd.texture_create(&format, &view);
            if texture_rid != Rid::Invalid {
                self.texture_rid = texture_rid;
                return;
            }
        }

        self.texture_rid = Rid::Invalid;
        godot_warn!(
            "ParcelRendererNode: failed to create output texture for STORAGE|SAMPLING usage."
        );
    }

    fn create_pipeline_and_uniforms(&mut self, rd: &mut Gd<RenderingDevice>) {
        if self.texture_rid == Rid::Invalid {
            godot_warn!("ParcelRendererNode: output texture RID is invalid.");
            return;
        }

        let shader_file: Gd<RdShaderFile> = load(SHADER_PATH);
        let Some(spirv) = shader_file.get_spirv() else {
            godot_warn!("ParcelRendererNode: failed to load SPIR-V from {}", SHADER_PATH);
            return;
        };

        self.shader_rid = rd.shader_create_from_spirv(&spirv);
        if self.shader_rid == Rid::Invalid {
            godot_warn!("ParcelRendererNode: failed to create shader RID.");
            return;
        }

        self.pipeline_rid = rd.compute_pipeline_create(self.shader_rid);
        if self.pipeline_rid == Rid::Invalid {
            godot_warn!("ParcelRendererNode: failed to create compute pipeline RID.");
            return;
        }

        let mut output_image = RdUniform::new_gd();
        output_image.set_uniform_type(UniformType::IMAGE);
        output_image.set_binding(0);
        output_image.add_id(self.texture_rid);

        self.uniform_set_rid = rd.uniform_set_create(&Array::from_iter([output_image]), self.shader_rid, 0);
        if self.uniform_set_rid == Rid::Invalid {
            godot_warn!(
                "ParcelRendererNode: failed to create uniform set (set=0, binding=0 image)."
            );
        }
    }

    fn attach_preview_texture(&mut self) {
        let mut texture = Texture2Drd::new_gd();
        texture.set_texture_rd_rid(self.texture_rid);

        if let Some(mut sprite) = self.base().try_get_node_as::<Sprite2D>(PREVIEW_NAME) {
            sprite.set_texture(&texture);
            sprite.set_centered(false);
            return;
        }

        let mut sprite = Sprite2D::new_alloc();
        sprite.set_name(PREVIEW_NAME);
        sprite.set_texture(&texture);
        sprite.set_centered(false);
        self.base_mut().add_child(&sprite);
    }

    fn set_preview_visible(&mut self, visible: bool) {
        if let Some(mut sprite) = self.base().try_get_node_as::<Sprite2D>(PREVIEW_NAME) {
            sprite.set_visible(visible);
        }
    }
}
