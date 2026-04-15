use godot::prelude::*;
use godot::classes::{
    RenderingDevice,
    RdTextureFormat,
    RdTextureView,
};
use godot::classes::rendering_device::DataFormat;
use godot::classes::rendering_device::TextureUsageBits;
use super::SplatViewer;

impl SplatViewer {
    pub fn create_texture(&mut self, rd: &mut Gd<RenderingDevice>) {
        let mut format = RdTextureFormat::new_gd();

        format.set_width(self.width);
        format.set_height(self.height);
        format.set_format(DataFormat::R32G32B32A32_SFLOAT);

        format.set_usage_bits(
            TextureUsageBits::STORAGE_BIT | TextureUsageBits::SAMPLING_BIT,
        );

        let view = RdTextureView::new_gd();

        self.texture_rid = rd.texture_create(&format, &view);
    }
}
