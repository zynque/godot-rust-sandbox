use godot::prelude::*;
use godot::classes::InputEvent;
use crate::physics::*;
use crate::input_state::{handle_input, InputState};

pub mod physics;
pub mod splat_shader;
pub mod lsystem;
pub mod input_state;

struct GodotRustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for GodotRustExtension {}

#[derive(GodotClass)]
#[class(base=Node2D)]
struct RustDrawing {
    #[base]
    input_state: InputState,
    physics: GodotPhysics,
    base: Base<Node2D>
}

#[godot_api]
impl INode2D for RustDrawing {
    fn init(base: Base<Node2D>) -> Self {
        godot_print!("RustDrawing initialized");

        let physics = new_physics_space()
            .expect("Failed to create physics space");

        Self {
            input_state: InputState::default(),
            physics,
            base,
        }
    }

    fn ready(&mut self) {
        let ok = crate::splat_shader::render_gaussian_splats(512);
        if !ok {
            godot_error!("Bindless RD: render_gaussian_splats failed");
        }
        self.base_mut().queue_redraw();
    }

    fn draw(&mut self) {
        let viewport = self.base().get_viewport_rect();
        let center = viewport.size * 0.5;
        let splats = crate::splat_shader::build_center_demo_splats(center);
        for splat in splats {
            self.base_mut().draw_circle(splat.position, splat.radius, splat.color);
        }

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
