use godot::prelude::*;
use crate::point_vec_extensions::PointVecExtensions;

pub mod point_vec_extensions;

struct GodotRustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for GodotRustExtension {}

#[derive(GodotClass)]
#[class(base=Node2D)]
struct RustCanvas {
    #[base]
    base: Base<Node2D>
}

#[godot_api]
impl INode2D for RustCanvas {
    fn init(base: Base<Node2D>) -> Self {
        godot_print!("RustCanvas initialized");

        Self { base }
    }

    fn draw(&mut self) {
        let points = vec![
            Vector2::new(0.0, -50.0),
            Vector2::new(50.0, 50.0),
            Vector2::new(-50.0, 50.0),
        ];

        let translated_points = points.translated(Vector2::new(100.0, 100.0));
        let packed_points = PackedVector2Array::from(translated_points);

        let color = Color::from_rgb(0.2, 0.8, 0.3);

        // Draw filled polygon
        self.base_mut().draw_colored_polygon(&packed_points, color);
    }
}
