use godot::prelude::*;
use crate::point_vec_extensions::PointVecExtensions;

pub fn dot() -> Vec<Vector2> {
    regular_polygon(24, 10.0)
}

pub fn regular_polygon(sides: usize, radius: f32) -> Vec<Vector2> {
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

pub fn copy_figure_at(figure: Vec<Vector2>, locations: Vec<Vector2>) -> Vec<Vec<Vector2>> {
    let mut copies = Vec::with_capacity(locations.len());

    for loc in locations {
        let translated_figure = figure.translated(loc);
        copies.push(translated_figure);
    }

    copies
}
