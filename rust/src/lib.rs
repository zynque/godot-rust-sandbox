use godot::prelude::*;
use godot::classes::{InputEventMouseButton, InputEvent};
use crate::point_vec_extensions::PointVecExtensions;

pub mod point_vec_extensions;

struct GodotRustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for GodotRustExtension {}

#[derive(GodotClass)]
#[class(base=Node2D)]
struct RustDrawing {
    #[base]
    angular_speed: f32,
    base: Base<Node2D>
}

#[godot_api]
impl INode2D for RustDrawing {
    fn init(base: Base<Node2D>) -> Self {
        godot_print!("RustDrawing initialized");

        Self {
            angular_speed: std::f32::consts::PI / 4.0,
            base,
        }
    }

    fn draw(&mut self) {
        let polygon = regular_polygon(8, 50.0);
        let locations = regular_polygon(8, 140.0);
        let replicated = copy_figure_at(polygon, locations);

        for points in replicated {
            let packed_points = PackedVector2Array::from(points);
            let color = Color::from_rgb(0.2, 0.7, 0.3);

            self.base_mut().draw_colored_polygon(&packed_points, color);
        }
    }

    fn physics_process(&mut self, delta: f32) {
      
        let radians = self.angular_speed * delta;
        self.base_mut().rotate(radians);
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

fn regular_polygon(sides: usize, radius: f32) -> Vec<Vector2> {
    let mut points = Vec::with_capacity(sides);
    let angle_step = std::f32::consts::TAU / sides as f32;

    for i in 0..sides {
        let angle = i as f32 * angle_step;
        let x = radius * angle.cos();
        let y = radius * angle.sin();
        points.push(Vector2::new(x, y));
    }

    points
}

fn copy_figure_at(figure: Vec<Vector2>, locations: Vec<Vector2>) -> Vec<Vec<Vector2>> {
    let mut copies = Vec::with_capacity(locations.len());

    for loc in locations {
        let translated_figure = figure.translated(loc);
        copies.push(translated_figure);
    }

    copies
}
