use godot::prelude::*;
use godot::classes::{InputEventMouseButton, InputEvent};
use crate::point_vec_extensions::PointVecExtensions;
use crate::physics::*;
use crate::bindless_rendering::GpuDot;

pub mod point_vec_extensions;
pub mod physics;
pub mod movement;
pub mod bindless_rendering;
pub mod figures;
use crate::figures::*;
struct GodotRustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for GodotRustExtension {}

#[derive(GodotClass)]
#[class(base=Node2D)]
struct RustDrawing {
    #[base]
    left_pressed: bool,
    right_pressed: bool,
    jump_pressed: bool,
    mouse_pressed: bool,
    mouse_position: Vector2,
    physics: GodotPhysics,
    gpu_dots: Vec<GpuDot>,
    base: Base<Node2D>
}

#[godot_api]
impl INode2D for RustDrawing {
    fn init(base: Base<Node2D>) -> Self {
        godot_print!("RustDrawing initialized");

        let mut physics = new_physics_space()
            .expect("Failed to create physics space");

        Self {
            left_pressed: false,
            right_pressed: false,
            jump_pressed: false,
            mouse_pressed: false,
            mouse_position: Vector2::ZERO,
            physics,
            gpu_dots: Vec::new(), // populated in ready()
            base,
        }
    }

    fn ready(&mut self) {
        self.gpu_dots = crate::bindless_rendering::compute_background_dots(512);
    }

    fn draw(&mut self) {
        // Draw GPU-computed background dots (bindless rendering test)
        let dots: Vec<(Vector2, f32, Color)> = self.gpu_dots
            .iter()
            .map(|d| (d.position, d.radius, d.color))
            .collect();
        for (pos, radius, color) in dots {
            self.base_mut().draw_circle(pos, radius, color);
        }

        // Draw the figure
        let color = Color::from_rgba(0.2, 0.8, 0.3, 1.0);
        let d = dot();
        let points = PackedVector2Array::from(d);
        self.base_mut().draw_colored_polygon(&points, color);
    }   

    fn input(&mut self, event: Gd<InputEvent>) {
        if event.is_action_pressed("move_left") {
            self.left_pressed = true;
        }
        if event.is_action_pressed("move_right") {
            self.right_pressed = true;
        }
        if event.is_action_released("move_left") {
            self.left_pressed = false;
        }
        if event.is_action_released("move_right") {
            self.right_pressed = false;
        }
        if event.is_action_pressed("jump") {
            self.jump_pressed = true;
        }
        if event.is_action_released("jump") {
            self.jump_pressed = false;
        }

        if let Ok(mouse_event) = event.try_cast::<InputEventMouseButton>() {
            if mouse_event.is_pressed() {
                self.mouse_pressed = true;
                self.mouse_position = mouse_event.get_position();
            } else {
                self.mouse_pressed = false;
            }
        }
    }
}

