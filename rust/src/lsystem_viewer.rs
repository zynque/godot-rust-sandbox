use godot::prelude::*;
use crate::lsystem::*;

#[derive(GodotClass)]
#[class(base=Node2D)]
struct LSystemViewer {
    #[base]
    base: Base<Node2D>
}

#[godot_api]
impl INode2D for LSystemViewer {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            base,
        }
    }

    fn draw(&mut self) {
        // Draw L-system: one rectangle per Forward segment (2 iterations, 20 px step, 4 px thick)
        let segments = lsystem_segments(2, 10.0, 4.0, Vector2::new(400.0, 300.0));
        let ls_color = Color::from_rgba(0.4, 0.6, 0.9, 0.85);
        for rect in segments {
            let packed = PackedVector2Array::from(rect.as_slice());
            self.base_mut().draw_colored_polygon(&packed, ls_color);
        }
    }   
}
