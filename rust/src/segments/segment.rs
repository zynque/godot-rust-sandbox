use godot::builtin::{
  Vector3
};

type SegmentId = usize;

struct Segments {
    parent: Vec<Option<SegmentId>>,
    children: Vec<Vec<SegmentId>>,
    direction: Vec<Vector3>,
    length: Vec<f32>,
}

impl Segments {
    pub fn new() -> Self {
        Self {
            parent: Vec::new(),
            children: Vec::new(),
            direction: Vec::new(),
            length: Vec::new(),
        }
    }
}
