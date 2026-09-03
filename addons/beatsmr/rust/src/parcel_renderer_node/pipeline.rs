use godot::classes::rendering_device::{DataFormat, TextureUsageBits, UniformType};
use godot::classes::{RdShaderFile, RdTextureFormat, RdTextureView, RdUniform, RenderingDevice};
use godot::prelude::*;

use super::{ParcelRendererNode, SHADER_PATH};

impl ParcelRendererNode {
    pub(super) fn destroy_resources(&mut self, rd: &mut Gd<RenderingDevice>) {
        if self.uniform_set_rid != Rid::Invalid {
            rd.free_rid(self.uniform_set_rid);
            self.uniform_set_rid = Rid::Invalid;
        }
        if self.pipeline_rid != Rid::Invalid {
            rd.free_rid(self.pipeline_rid);
            self.pipeline_rid = Rid::Invalid;
        }
        if self.shader_rid != Rid::Invalid {
            rd.free_rid(self.shader_rid);
            self.shader_rid = Rid::Invalid;
        }
        if self.texture_rid != Rid::Invalid {
            rd.free_rid(self.texture_rid);
            self.texture_rid = Rid::Invalid;
        }
    }

    pub(super) fn recreate_resources(&mut self, rd: &mut Gd<RenderingDevice>) {
        self.destroy_resources(rd);
        self.create_texture(rd);
        self.create_pipeline_and_uniforms(rd);
    }

    pub(super) fn create_texture(&mut self, rd: &mut Gd<RenderingDevice>) {
        let usage = TextureUsageBits::STORAGE_BIT | TextureUsageBits::SAMPLING_BIT;
        let data_format = DataFormat::R32G32B32A32_SFLOAT;

        if !rd.texture_is_format_supported_for_usage(data_format, usage) {
            self.texture_rid = Rid::Invalid;
            godot_warn!(
                "ParcelRendererNode: R32G32B32A32_SFLOAT not supported for STORAGE|SAMPLING usage."
            );
            return;
        }

        let mut format = RdTextureFormat::new_gd();
        format.set_width(self.width);
        format.set_height(self.height);
        format.set_format(data_format);
        format.set_usage_bits(usage);

        let view = RdTextureView::new_gd();
        self.texture_rid = rd.texture_create(&format, &view);
        if self.texture_rid != Rid::Invalid {
            return;
        }

        self.texture_rid = Rid::Invalid;
        godot_warn!("ParcelRendererNode: failed to create RGBA32F output texture.");
    }

    pub(super) fn create_pipeline_and_uniforms(&mut self, rd: &mut Gd<RenderingDevice>) {
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

        self.uniform_set_rid =
            rd.uniform_set_create(&Array::from_iter([output_image]), self.shader_rid, 0);
        if self.uniform_set_rid == Rid::Invalid {
            godot_warn!(
                "ParcelRendererNode: failed to create uniform set (set=0, binding=0 image)."
            );
        }
    }
}
