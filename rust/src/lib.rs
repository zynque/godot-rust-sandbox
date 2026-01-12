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
    dot_position: Vector2,
    moving_by: Vector2,
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
            dot_position: Vector2::ZERO,
            moving_by: Vector2::ZERO,
            base,
        }
    }

    fn draw(&mut self) {
        let color = Color::from_rgba(0.2, 0.8, 0.3, 1.0);
        let f = figure();
        for poly in f {
            let points = PackedVector2Array::from(poly.translated(self.dot_position));
            self.base_mut().draw_colored_polygon(&points, color);
        }
    }   

    fn physics_process(&mut self, _delta: f32) {
        // let radians = self.angular_speed * delta;
        // self.base_mut().rotate(radians);
        self.dot_position += self.moving_by;
        self.base_mut().queue_redraw();
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if event.is_action_pressed("move_left") {
            self.moving_by = Vector2::new(-5.0, 0.0);
        }
        if event.is_action_pressed("move_right") {
            self.moving_by = Vector2::new(5.0, 0.0);
        }
        if event.is_action_released("move_left") {
            self.moving_by = Vector2::ZERO;
        }
        if event.is_action_released("move_right") {
            self.moving_by = Vector2::ZERO;
        }

        if let Ok(mouse_event) = event.try_cast::<InputEventMouseButton>() {
            if mouse_event.is_pressed() {
                self.dot_position = mouse_event.get_position();
                godot_print!("Dot moved to: {:?}", self.dot_position);
            }
        }
    }
}

fn figure() -> Vec<Vec<Vector2>> {
    let d = dot();
    let locations = vec![
        Vector2::new(-12.0, 20.0),
        Vector2::new(12.0, 20.0),
        Vector2::new(-12.0, 0.0),
        Vector2::new(12.0, 0.0),
        Vector2::new(0.0, -20.0),
        Vector2::new(-12.0, -40.0),
        Vector2::new(12.0, -40.0),
        Vector2::new(-32.0, -40.0),
        Vector2::new(32.0, -40.0),
        Vector2::new(0.0, -65.0),
    ];

    copy_figure_at(d, locations)
}

fn dot() -> Vec<Vector2> {
    regular_polygon(24,10.0)
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
