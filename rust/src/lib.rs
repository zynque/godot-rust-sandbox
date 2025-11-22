use godot::prelude::*;
use godot::classes::{ISprite2D, InputEventMouseButton, Sprite2D};
use godot::classes::{InputEvent};
use crate::point_vec_extensions::PointVecExtensions;

pub mod point_vec_extensions;

struct GodotRustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for GodotRustExtension {}

#[derive(GodotClass)]
#[class(base=Node2D)]
struct RustCanvas {
    #[base]
    angular_speed: f64,
    base: Base<Node2D>
}

#[godot_api]
impl INode2D for RustCanvas {
    fn init(base: Base<Node2D>) -> Self {
        godot_print!("RustCanvas initialized");

        Self {
            angular_speed: std::f64::consts::PI,
            base,
        }
    }

    fn draw(&mut self) {
        let points = vec![
            Vector2::new(0.0, -50.0),
            Vector2::new(50.0, 50.0),
            Vector2::new(-50.0, 50.0),
        ];

        let translated_points = points.translated(Vector2::new(10.0, 10.0));
        let packed_points = PackedVector2Array::from(translated_points);

        let color = Color::from_rgb(0.2, 0.8, 0.3);

        // Draw filled polygon
        self.base_mut().draw_colored_polygon(&packed_points, color);
    }

    fn physics_process(&mut self, delta: f64) {
        // In GDScript, this would be: 
        // rotation += angular_speed * delta
        
        let radians = (self.angular_speed * delta) as f32;
        self.base_mut().rotate(radians);
        // The 'rotate' method requires a f32, 
        // therefore we convert 'self.angular_speed * delta' which is a f64 to a f32
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if let Ok(mouse_event) = event.try_cast::<InputEventMouseButton>() {
            if mouse_event.is_pressed() {
                let p = mouse_event.get_position();
                self.base_mut().set_position(p);
            }
        }
    }
}

#[derive(GodotClass)]
#[class(base=Sprite2D)]
struct Player {
    angular_speed: f64,
    base: Base<Sprite2D>
}

#[godot_api]
impl ISprite2D for Player {
    fn init(base: Base<Sprite2D>) -> Self {
        godot_print!("Hello, world!"); // Prints to the Godot console
        
        Self {
            angular_speed: std::f64::consts::PI,
            base,
        }
    }

    fn physics_process(&mut self, delta: f64) {
        // In GDScript, this would be: 
        // rotation += angular_speed * delta
        
        let radians = (self.angular_speed * delta) as f32;
        self.base_mut().rotate(radians);
        // The 'rotate' method requires a f32, 
        // therefore we convert 'self.angular_speed * delta' which is a f64 to a f32
    }
}
