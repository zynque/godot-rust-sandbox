use godot::prelude::*;
use godot::classes::{InputEventMouseButton, InputEvent};
use crate::point_vec_extensions::PointVecExtensions;
use crate::physics::*;
use crate::movement::{move_and_slide, MoveAndSlideParams};
use crate::bindless_rendering::GpuDot;

pub mod point_vec_extensions;
pub mod physics;
pub mod movement;
pub mod bindless_rendering;
struct GodotRustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for GodotRustExtension {}

#[derive(GodotClass)]
#[class(base=Node2D)]
struct RustDrawing {
    #[base]
    dot_position: Vector2,
    left_pressed: bool,
    right_pressed: bool,
    velocity: Vector2,
    on_floor: bool,
    physics: GodotPhysics,
    body_rid: Rid,
    collision_areas: Vec<(Vec<Vector2>, Vector2)>, // (polygon, position) for rendering
    gpu_dots: Vec<GpuDot>,
    base: Base<Node2D>
}

#[godot_api]
impl INode2D for RustDrawing {
    fn init(base: Base<Node2D>) -> Self {
        godot_print!("RustDrawing initialized");

        let mut physics = new_physics_space()
            .expect("Failed to create physics space");
        let mut collision_areas: Vec<(Vec<Vector2>, Vector2)> = Vec::new();

        // Default Godot 4 viewport: 1152 × 648
        const W: f32 = 1152.0;
        const H: f32 = 648.0;
        const WALL: f32 = 20.0;
        let floor_y = H - WALL; // 628  — top of the floor surface

        // Helper: register a static body and remember it for drawing
        let add = |physics: &mut GodotPhysics,
                       areas: &mut Vec<(Vec<Vector2>, Vector2)>,
                       poly: Vec<Vector2>,
                       pos: Vector2| {
            physics.add_static_body_polygon(&poly, pos);
            areas.push((poly, pos));
        };

        // --- Floor (full-width, 20 px thick at the very bottom) ---
        add(&mut physics, &mut collision_areas,
            vec![Vector2::new(0.0, 0.0), Vector2::new(W, 0.0),
                 Vector2::new(W, WALL),  Vector2::new(0.0, WALL)],
            Vector2::new(0.0, floor_y));

        // --- Left wall (full-height, flush with left edge) ---
        add(&mut physics, &mut collision_areas,
            vec![Vector2::new(0.0, 0.0), Vector2::new(WALL, 0.0),
                 Vector2::new(WALL, H),  Vector2::new(0.0, H)],
            Vector2::ZERO);

        // --- Right wall (full-height, flush with right edge) ---
        add(&mut physics, &mut collision_areas,
            vec![Vector2::new(0.0, 0.0), Vector2::new(WALL, 0.0),
                 Vector2::new(WALL, H),  Vector2::new(0.0, H)],
            Vector2::new(W - WALL, 0.0));

        // ===== Obstacles (all y-values relative to floor_y) =====

        // 1) Three ascending steps (25 / 50 / 80 px tall)
        //    Bottom extends WALL-deep into the floor to eliminate seam gaps.
        for (i, h) in [25.0_f32, 50.0, 80.0].iter().enumerate() {
            let x = 120.0 + i as f32 * 70.0;
            add(&mut physics, &mut collision_areas,
                vec![Vector2::new(0.0, WALL), Vector2::new(60.0, WALL),
                     Vector2::new(60.0, -*h), Vector2::new(0.0, -*h)],
                Vector2::new(x, floor_y));
        }

        // 2) Gentle ramp ~15° (200 wide × 55 tall, going up left→right)
        //    Quad with base WALL-deep so only the slope is exposed above floor.
        add(&mut physics, &mut collision_areas,
            vec![Vector2::new(0.0, WALL), Vector2::new(200.0, WALL),
                 Vector2::new(200.0, -55.0), Vector2::new(0.0, 2.0)],
            Vector2::new(380.0, floor_y));

        // 3) Small platform on top of the gentle ramp
        add(&mut physics, &mut collision_areas,
            vec![Vector2::new(0.0, 10.0), Vector2::new(80.0, 10.0),
                 Vector2::new(80.0, -15.0), Vector2::new(0.0, -15.0)],
            Vector2::new(580.0, floor_y - 55.0));

        // 4) 45° ramp (80 × 80, going up left→right)
        add(&mut physics, &mut collision_areas,
            vec![Vector2::new(0.0, WALL), Vector2::new(80.0, WALL),
                 Vector2::new(80.0, -80.0), Vector2::new(0.0, 2.0)],
            Vector2::new(700.0, floor_y));

        // 5) Steep ramp ~60° (50 wide × 87 tall, going up left→right)
        add(&mut physics, &mut collision_areas,
            vec![Vector2::new(0.0, WALL), Vector2::new(50.0, WALL),
                 Vector2::new(50.0, -87.0), Vector2::new(0.0, 2.0)],
            Vector2::new(820.0, floor_y));

        // 6) Down-ramp ~18° (going DOWN left→right, 150 wide × 50 tall)
        add(&mut physics, &mut collision_areas,
            vec![Vector2::new(0.0, WALL), Vector2::new(150.0, WALL),
                 Vector2::new(150.0, 2.0), Vector2::new(0.0, -50.0)],
            Vector2::new(920.0, floor_y));

        // 7) Floating platform (upper area, for jump testing)
        add(&mut physics, &mut collision_areas,
            vec![Vector2::new(0.0, 0.0), Vector2::new(140.0, 0.0),
                 Vector2::new(140.0, 15.0), Vector2::new(0.0, 15.0)],
            Vector2::new(500.0, 380.0));

        godot_print!("Level geometry created ({} bodies)", collision_areas.len());

        let dot_position = Vector2::new(100.0, 100.0);
        let body_rid = physics.create_body(&dot(), dot_position);

        Self {
            dot_position,
            left_pressed: false,
            right_pressed: false,
            velocity: Vector2::ZERO,
            on_floor: false,
            physics,
            body_rid,
            collision_areas,
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

        // Draw collision areas (floor and walls)
        let wall_color = Color::from_rgba(0.5, 0.5, 0.5, 1.0);
        let wall_polygons: Vec<PackedVector2Array> = self.collision_areas
            .iter()
            .map(|(polygon, position)| PackedVector2Array::from(polygon.translated(*position)))
            .collect();
        
        for points in wall_polygons {
            self.base_mut().draw_colored_polygon(&points, wall_color);
        }

        // Draw the figure
        let color = Color::from_rgba(0.2, 0.8, 0.3, 1.0);
        let f = figure();
        for poly in f {
            let points = PackedVector2Array::from(poly.translated(self.dot_position));
            self.base_mut().draw_colored_polygon(&points, color);
        }
    }   

    fn physics_process(&mut self, delta: f32) {
        const GRAVITY: f32 = 980.0;
        const MOVE_SPEED: f32 = 200.0;

        let was_on_floor = self.on_floor;

        // --- Build overall velocity vector from input + gravity ---
        self.velocity.x = if self.left_pressed && !self.right_pressed {
            -MOVE_SPEED
        } else if self.right_pressed && !self.left_pressed {
            MOVE_SPEED
        } else {
            0.0
        };

        self.velocity.y += GRAVITY * delta;

        // Cap fall speed to avoid tunnelling through thin geometry
        self.velocity.y = self.velocity.y.min(2000.0);

        // --- Move-and-slide ---
        self.on_floor = move_and_slide(
            &mut self.physics,
            self.body_rid,
            &mut self.dot_position,
            &mut self.velocity,
            was_on_floor,
            delta,
            &MoveAndSlideParams::default(),
        );

        // Sync the kinematic body's transform for the next frame
        self.physics.set_body_position(self.body_rid, self.dot_position);

        self.base_mut().queue_redraw();
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
        
        if event.is_action_pressed("jump") && self.on_floor {
            self.velocity.y = -400.0;
        }

        if let Ok(mouse_event) = event.try_cast::<InputEventMouseButton>() {
            if mouse_event.is_pressed() {
                self.dot_position = mouse_event.get_position();
                self.velocity = Vector2::ZERO;
                self.physics.set_body_position(self.body_rid, self.dot_position);
                godot_print!("Dot moved to: {:?}", self.dot_position);
            }
        }
    }
}

fn figure() -> Vec<Vec<Vector2>> {
    let d = dot();
    let locations = vec![
        Vector2::new(0.0, 0.0),
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
