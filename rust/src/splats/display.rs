use godot::prelude::*;
use godot::classes::{Shader, ShaderMaterial, Texture2Drd, TextureRect};
use super::SplatViewer;

impl SplatViewer {
    pub fn display_result(&mut self) {
        let mut tex = Texture2Drd::new_gd();
        tex.set_texture_rd_rid(self.texture_rid);

        let shader: Gd<Shader> = load("res://display_shader.gdshader");
        let mut material = ShaderMaterial::new_gd();
        material.set_shader(&shader);
        material.set_shader_parameter("render_texture", &tex.to_variant());

        let size = Vector2::new(self.width as f32, self.height as f32);

        if let Some(mut rect) = self
            .base()
            .try_get_node_as::<TextureRect>("../SplatPreview")
            .or_else(|| self.base().try_get_node_as::<TextureRect>("SplatPreview"))
        {
            rect.set_texture(&tex);
            rect.set_material(&material);
            rect.set_position(Vector2::ZERO);
            rect.set_size(size);
            rect.set_custom_minimum_size(size);
            rect.set_visible(true);
            return;
        }

        let mut rect = TextureRect::new_alloc();
        rect.set_name("SplatPreview");
        rect.set_texture(&tex);
        rect.set_material(&material);
        rect.set_position(Vector2::ZERO);
        rect.set_size(size);
        rect.set_custom_minimum_size(size);
        rect.set_visible(true);

        if let Some(mut parent) = self.base().get_parent() {
            parent.call_deferred("add_child", &[rect.to_variant()]);
        } else {
            self.base_mut().call_deferred("add_child", &[rect.to_variant()]);
        }
    }
}
