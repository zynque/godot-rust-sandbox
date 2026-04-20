use godot::builtin::{
  Vector3
};

pub type SegmentId = usize;

// Each segment is a piece of a growing plant. You can think of these as 'cells' that make up the plant's structure.
// Each has a roughly cylindrical shape defined by its direction and length.
// We use a Structure of Arrays for better cache performance.
pub struct Segments {
    parent: Vec<Option<SegmentId>>,
    first_child: Vec<Option<SegmentId>>,
    next_sibling: Vec<Option<SegmentId>>,
    direction: Vec<Vector3>,
    length: Vec<f32>,
}

impl Segments {
    pub fn new() -> Self {
        Self {
            parent: Vec::new(),
            first_child: Vec::new(),
            next_sibling: Vec::new(),
            direction: Vec::new(),
            length: Vec::new(),
        }
    }

    pub fn push_root(&mut self, direction: Vector3, length: f32) -> SegmentId {
        let id = self.direction.len();
        self.parent.push(None);
        self.first_child.push(None);
        self.next_sibling.push(None);
        self.direction.push(direction);
        self.length.push(length);
        id
    }

    pub fn direction(&self, id: SegmentId) -> Option<Vector3> {
        self.direction.get(id).copied()
    }

    pub fn length(&self, id: SegmentId) -> Option<f32> {
        self.length.get(id).copied()
    }
}
