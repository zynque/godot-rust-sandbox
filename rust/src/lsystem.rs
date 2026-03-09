use std::f32::consts::TAU;

use godot::prelude::*;

/// Symbols in the L-system alphabet.
#[derive(Clone, Debug)]
pub enum Op {
    /// Move forward one step and record the position.
    Forward,
    /// Turn left (counter-clockwise) by 90 degrees.
    TurnLeft,
    /// Turn right (clockwise) by 90 degrees.
    TurnRight,
}

pub struct Seed {
    pub angle: f32,
    pub ops: Vec<Op>,
}

/// The seed / axiom: FLFRFRFFLFLFRF
pub fn build_axiom() -> Seed {
    SeedBuilder::new(0.21)
        .f().l().f().r().f().r().f().f().l().f().l().f().r().f()
        .build()
}

pub struct SeedBuilder {
    pub angle: f32,
    pub ops: Vec<Op>,
}

impl SeedBuilder {
    fn new(angle: f32) -> Self {
        Self { angle, ops: Vec::new() }
    }

    fn build(&self) -> Seed {
        Seed{ angle: self.angle, ops: self.ops.clone() }
    }

    fn f(mut self) -> Self {
        self.ops.push(Op::Forward);
        self
    }

    fn l(mut self) -> Self {
        self.ops.push(Op::TurnLeft);
        self
    }

    fn r(mut self) -> Self {
        self.ops.push(Op::TurnRight);
        self
    }
}

/// Apply the production rule `F → AXIOM` the given number of times.
/// `L` and `R` are terminals and are passed through unchanged.
pub fn expand(seed: Seed, iterations: usize) -> Seed {
    let original_length = seed.ops.len();
    let mut current = seed.ops.clone();

    for _ in 0..iterations {
        let mut next: Vec<Op> = Vec::with_capacity(current.len() * original_length);
        for op in &current {
            match op {
                Op::Forward => next.extend_from_slice(&seed.ops),
                Op::TurnLeft => next.push(Op::TurnLeft),
                Op::TurnRight => next.push(Op::TurnRight),
            }
        }
        current = next;
    }

    Seed { angle: seed.angle, ops: current }
}

/// Walk the op sequence with a turtle, returning one rectangle (4 corners,
/// clockwise) for every `Forward` step.
/// `length` is the length of each segment; `thickness` is the stroke width.
/// The turtle starts at (0, 0) facing right (+X).
pub fn segment_rectangles(angle: f32, ops: &[Op], length: f32, thickness: f32) -> Vec<[Vector2; 4]> {
    let mut pos = Vector2::ZERO;
    let mut dir = Vector2::new(1.0, 0.0);
    let half = thickness * 0.5;
    let mut rects = Vec::new();

    for op in ops {
        match op {
            Op::Forward => {
                let next = pos + dir * length;
                // Perpendicular to direction (both sides).
                let perp = Vector2::new(-dir.y, dir.x) * half;
                // Four corners of the rectangle, clockwise.
                let rect = [
                    pos + perp,
                    next + perp,
                    next - perp,
                    pos - perp,
                ];
                rects.push(rect);
                pos = next;
            }
            Op::TurnLeft => {
                dir = dir.rotated(-TAU * angle);
            }
            Op::TurnRight => {
                dir = dir.rotated(TAU * angle);
            }
        }
    }

    rects
}

/// Collect all rectangle corners to compute the bounding box centre.
fn bounding_centre(rects: &[[Vector2; 4]]) -> Vector2 {
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for rect in rects {
        for p in rect {
            if p.x < min_x { min_x = p.x; }
            if p.x > max_x { max_x = p.x; }
            if p.y < min_y { min_y = p.y; }
            if p.y > max_y { max_y = p.y; }
        }
    }
    Vector2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5)
}

/// Expand the axiom `iterations` times and return one rectangle per Forward
/// length, centred around `origin`.
/// `length` is the segment length; `thickness` is the rectangle width.
pub fn lsystem_segments(
    iterations: usize,
    length: f32,
    thickness: f32,
    origin: Vector2,
) -> Vec<[Vector2; 4]> {
    let seed = expand(build_axiom(), iterations);
    let mut rects = segment_rectangles(seed.angle, &seed.ops, length, thickness);
    let centre = bounding_centre(&rects);
    let offset = origin - centre;
    for rect in &mut rects {
        for p in rect.iter_mut() {
            *p += offset;
        }
    }
    rects
}

struct GrowthSegment<T> {
    pub value: T,
    pub branches: Vec<GrowthSegment<T>>,
    pub continuation: Option<Box<GrowthSegment<T>>>,
}

type growth_function<T> = fn(node: GrowthSegment<T>) -> GrowthSegment<T>;
