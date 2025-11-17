use godot::prelude::*;
use godot::classes::{Sprite2D, ISprite2D};

struct GodotRustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for GodotRustExtension {}

#[derive(GodotClass)]
#[class(base=Sprite2D)]
struct Player {
    angular_speed: f32,

    #[base]
    base: Base<Sprite2D>
}

#[godot_api]
impl ISprite2D for Player {
    fn init(base: Base<Sprite2D>) -> Self {
        godot_print!("Player initialized");

        Self {
            angular_speed: std::f32::consts::PI,
            base
        }
    }

    fn physics_process(&mut self, delta: f32) {
        let radians = self.angular_speed * delta;
        self.base_mut().rotate(radians);
    }
}

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

        let translated_points = translate_points(&points, Vector2::new(100.0, 100.0));
        let packed_points = PackedVector2Array::from(translated_points);

        let color = Color::from_rgb(0.2, 0.8, 0.3);

        // Draw filled polygon
        self.base_mut().draw_colored_polygon(&packed_points, color);
    }
}

fn translate_points(points: &Vec<Vector2>, offset: Vector2) -> Vec<Vector2> {
    points.iter().map(|point| *point + offset).collect::<Vec<_>>()
}
