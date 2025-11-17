use godot::prelude::Vector2;

pub trait PointVecExtensions {
    fn translated(&self, offset: Vector2) -> Vec<Vector2>;
}

impl PointVecExtensions for Vec<Vector2> {
    fn translated(&self, offset: Vector2) -> Vec<Vector2> {
        self.iter().map(|point| *point + offset).collect::<Vec<_>>()
    }
}
