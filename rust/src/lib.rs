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
    physics: GodotPhysics,
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
        physics.add_area_polygon(&floor_polygon, floor_position);
        collision_areas.push((floor_polygon, floor_position));

        // Create left wall
        let left_wall = vec![
            Vector2::new(0.0, 0.0),
            Vector2::new(50.0, 0.0),
            Vector2::new(50.0, 600.0),
            Vector2::new(0.0, 600.0),
        ];
        let left_wall_position = Vector2::new(0.0, 0.0);
        physics.add_area_polygon(&left_wall, left_wall_position);
        collision_areas.push((left_wall, left_wall_position));

        // Create right wall
        let right_wall = vec![
            Vector2::new(0.0, 0.0),
            Vector2::new(50.0, 0.0),
            Vector2::new(50.0, 600.0),
            Vector2::new(0.0, 600.0),
        ];
        let right_wall_position = Vector2::new(590.0, 0.0);
        physics.add_area_polygon(&right_wall, right_wall_position);
        collision_areas.push((right_wall, right_wall_position));

        godot_print!("Floor and walls created");

        Self {
            dot_position: Vector2::new(320.0, 100.0), // Start above the floor
            left_pressed: false,
            right_pressed: false,
            velocity: Vector2::ZERO,
            physics,
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
        // Update horizontal velocity based on key states
        let move_speed = 200.0; // pixels per second
        if self.left_pressed && !self.right_pressed {
            self.velocity.x = -move_speed;
        } else if self.right_pressed && !self.left_pressed {
            self.velocity.x = move_speed;
        } else {
            self.velocity.x = 0.0;
        }

        // Apply gravity
        let gravity = Vector2::new(0.0, 980.0); // pixels per second^2
        self.velocity.y += gravity.y * delta;

        // Calculate movement based on velocity
        let desired_movement = self.velocity * delta;

        // Move as far as possible before collision
        let figure_poly = dot();
        let result = self.physics.cast_motion(&figure_poly, self.dot_position, desired_movement);
        
        // Apply the safe movement
        self.dot_position += result.motion;
        
        // If we collided, try to slide along the surface
        if result.collided && result.remainder.length_squared() > 0.01 {
            // Try to slide along the collision surface
            // If we hit floor/ceiling, allow horizontal movement
            if result.remainder.y.abs() > result.remainder.x.abs() {
                // Primarily vertical collision - try horizontal slide
                let slide_movement = Vector2::new(result.remainder.x, 0.0);
                let slide_result = self.physics.cast_motion(&figure_poly, self.dot_position, slide_movement);
                self.dot_position += slide_result.motion;
                
                // Stop vertical velocity when hitting floor or ceiling
                if self.velocity.y > 0.0 && result.remainder.y > 0.0 {
                    self.velocity.y = 0.0;
                } else if self.velocity.y < 0.0 && result.remainder.y < 0.0 {
                    self.velocity.y = 0.0;
                }
            } else {
                // Primarily horizontal collision - try vertical slide
                let slide_movement = Vector2::new(0.0, result.remainder.y);
                let slide_result = self.physics.cast_motion(&figure_poly, self.dot_position, slide_movement);
                self.dot_position += slide_result.motion;
                
                // Stop horizontal velocity when hitting walls
                self.velocity.x = 0.0;
            }
        }

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
        
        if event.is_action_pressed("jump") {
            // Apply upward impulse
            self.velocity.y = -400.0; // Negative Y is upward in Godot
            godot_print!("Jump!");
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
