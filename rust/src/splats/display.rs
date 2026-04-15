use godot::prelude::*;
use godot::classes::{Texture2Drd, TextureRect};
use super::SplatViewer;

impl SplatViewer {
    pub fn display_result(&mut self) {
        let mut tex = Texture2Drd::new_gd();
        tex.set_texture_rd_rid(self.texture_rid);

        let mut rect = TextureRect::new_alloc();
        rect.set_texture(&tex);

        rect.set_custom_minimum_size(
            Vector2::new(self.width as f32, self.height as f32)
        );

        self.base_mut().add_child(&rect);
    }
}
