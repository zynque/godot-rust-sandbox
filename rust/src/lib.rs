use godot::prelude::*;
use godot::classes::InputEvent;
use crate::point_vec_extensions::PointVecExtensions;
use crate::physics::*;
use crate::bindless_rendering::GpuDot;
use crate::input_state::{handle_input, InputState};

pub mod point_vec_extensions;
pub mod physics;
pub mod movement;
pub mod bindless_rendering;
pub mod figures;
pub mod lsystem;
pub mod input_state;
use crate::figures::*;
struct GodotRustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for GodotRustExtension {}

#[derive(GodotClass)]
#[class(base=Node2D)]
struct RustDrawing {
    #[base]
    input_state: InputState,
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
            input_state: InputState::default(),
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

        // Draw L-system: one rectangle per Forward segment (2 iterations, 20 px step, 4 px thick)
        let segments = lsystem::lsystem_segments(2, 10.0, 4.0, Vector2::new(400.0, 300.0));
        let ls_color = Color::from_rgba(0.4, 0.6, 0.9, 0.85);
        for rect in segments {
            let packed = PackedVector2Array::from(rect.as_slice());
            self.base_mut().draw_colored_polygon(&packed, ls_color);
        }
    }   

    fn input(&mut self, event: Gd<InputEvent>) {
        handle_input(event, &mut self.input_state);
    }
}

