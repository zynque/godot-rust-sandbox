use godot::classes::{Sprite2D, Texture2Drd};
use godot::prelude::*;

use super::{ParcelRendererNode, PREVIEW_NAME};

impl ParcelRendererNode {
    pub(super) fn attach_preview_texture(&mut self) {
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

    pub(super) fn set_preview_visible(&mut self, visible: bool) {
        if let Some(mut sprite) = self.base().try_get_node_as::<Sprite2D>(PREVIEW_NAME) {
            sprite.set_visible(visible);
        }
    }
}
