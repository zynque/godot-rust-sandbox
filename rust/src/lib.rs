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
    left_pressed: bool,
    right_pressed: bool,
    velocity: Vector2,
    on_floor: bool,
    physics: GodotPhysics,
    body_rid: Rid,
    collision_areas: Vec<(Vec<Vector2>, Vector2)>, // (polygon, position) for rendering
    base: Base<Node2D>
}

#[godot_api]
impl INode2D for RustDrawing {
    fn init(base: Base<Node2D>) -> Self {
        godot_print!("RustDrawing initialized");

        let mut physics = new_physics_space()
            .expect("Failed to create physics space");
        let mut collision_areas = Vec::new();

        // Create floor as a wide rectangle at the bottom of the screen
        let floor_polygon = vec![
            Vector2::new(-500.0, 0.0),
            Vector2::new(500.0, 0.0),
            Vector2::new(500.0, 50.0),
            Vector2::new(-500.0, 50.0),
        ];
        let floor_position = Vector2::new(320.0, 500.0);
        physics.add_static_body_polygon(&floor_polygon, floor_position);
        collision_areas.push((floor_polygon, floor_position));

        // Create left wall
        let left_wall = vec![
            Vector2::new(0.0, 0.0),
            Vector2::new(50.0, 0.0),
            Vector2::new(50.0, 600.0),
            Vector2::new(0.0, 600.0),
        ];
        let left_wall_position = Vector2::new(0.0, 0.0);
        physics.add_static_body_polygon(&left_wall, left_wall_position);
        collision_areas.push((left_wall, left_wall_position));

        // Create right wall
        let right_wall = vec![
            Vector2::new(0.0, 0.0),
            Vector2::new(50.0, 0.0),
            Vector2::new(50.0, 600.0),
            Vector2::new(0.0, 600.0),
        ];
        let right_wall_position = Vector2::new(590.0, 0.0);
        physics.add_static_body_polygon(&right_wall, right_wall_position);
        collision_areas.push((right_wall, right_wall_position));

        godot_print!("Floor and walls created");

        let dot_position = Vector2::new(320.0, 100.0);
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
            base,
        }
    }

    fn draw(&mut self) {
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
        // --- Constants (mirroring Godot's CharacterBody2D defaults) ---
        const MAX_SLIDES: i32 = 6;
        const FLOOR_MAX_ANGLE_COS: f32 = 0.70710678; // cos(45°)
        const GRAVITY: f32 = 980.0;
        const MOVE_SPEED: f32 = 200.0;
        const FLOOR_SNAP_LENGTH: f32 = 4.0;

        let up_direction = Vector2::UP;
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

        // --- Move-and-slide loop (modelled on CharacterBody2D) ---
        let mut motion = self.velocity * delta;
        self.on_floor = false;

        for _slide in 0..MAX_SLIDES {
            if motion.length_squared() < 1e-6 {
                break;
            }

            let result = self.physics.body_test_motion(
                self.body_rid,
                self.dot_position,
                motion,
            );

            if !result.collided {
                self.dot_position += motion;
                break;
            }

            // Apply the safe portion of the motion
            self.dot_position += result.travel;

            let normal = result.collision_normal;

            // Classify surface by comparing normal with up direction
            if normal.dot(up_direction) > FLOOR_MAX_ANGLE_COS {
                self.on_floor = true;
            }

            // Slide the remaining motion along the collision surface
            //   slide(v, n) = v − (v · n) × n
            motion = result.remainder
                - normal * result.remainder.dot(normal);

            // Slide velocity so it doesn't accumulate into the surface
            let vel_into_surface = self.velocity.dot(normal);
            if vel_into_surface < 0.0 {
                self.velocity -= normal * vel_into_surface;
            }
        }

        // Clamp near-zero velocity components to avoid drift
        if self.velocity.x.abs() < 0.001 {
            self.velocity.x = 0.0;
        }
        if self.velocity.y.abs() < 0.001 {
            self.velocity.y = 0.0;
        }

        // --- Floor snap (keeps body grounded over small bumps / edges) ---
        if was_on_floor && !self.on_floor && self.velocity.y >= 0.0 {
            let snap_result = self.physics.body_test_motion(
                self.body_rid,
                self.dot_position,
                Vector2::new(0.0, FLOOR_SNAP_LENGTH),
            );
            if snap_result.collided {
                let snap_normal = snap_result.collision_normal;
                if snap_normal.dot(up_direction) > FLOOR_MAX_ANGLE_COS {
                    self.dot_position += snap_result.travel;
                    self.on_floor = true;
                }
            }
        }

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
