use godot::prelude::*;
use godot::classes::{InputEventMouseButton, InputEvent};
use crate::point_vec_extensions::PointVecExtensions;
use crate::physics::*;

pub mod point_vec_extensions;
pub mod physics;
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

        let mut physics = new_physics_space().unwrap();

        let polygon = regular_polygon(4, 30.0);
        let position = Vector2::ZERO;
        let position2 = Vector2::new(59.99, 0.0);
        physics.add_area_polygon(&polygon, position);
        let collides = physics.polygon_collides(&polygon, position2);

        godot_print!("Polygon collides: {}", collides);

        Self {
            angular_speed: std::f32::consts::PI / 4.0,
            base,
        }
    }

    fn draw(&mut self) {
        let polygon: Vec<Vector2> = regular_polygon(5, 30.0);
        let locations = regular_polygon(8, 80.0);
        let copies = copy_figure_at(polygon, locations);
        let color = Color::from_rgba(0.2, 0.7, 0.3, 0.5);

        for polygon in copies {
            let packed_points = PackedVector2Array::from(polygon);
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
    let angle_step: f32 = std::f32::consts::TAU / sides as f32;

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
