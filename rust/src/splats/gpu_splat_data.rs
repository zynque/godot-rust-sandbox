use godot::builtin::Vector3;

use crate::segments::Segments;

#[repr(C)]
pub struct GpuSplatData {
    pub position: [f32; 3],
    pub density: f32,

    pub cov0: [f32; 3],
    pub cov1: [f32; 3],
    pub cov2: [f32; 3],

    pub color: [f32; 3],
    pub pad: f32,
}

impl GpuSplatData {
    pub fn from_segment(direction: Vector3, length: f32) -> Self {
        let safe_dir = if direction.length_squared() > 0.0 {
            direction.normalized()
        } else {
            Vector3::UP
        };

        let tip = safe_dir * length;

        Self {
            position: [tip.x, tip.y, tip.z],
            density: 1.0,
            cov0: [0.010, 0.0, 0.0],
            cov1: [0.0, 0.010, 0.0],
            cov2: [0.0, 0.0, 0.010],
            color: [0.20, 0.95, 0.35],
            pad: 0.0,
        }
    }

    fn as_floats(&self) -> [f32; 20] {
        [
            self.position[0], self.position[1], self.position[2], self.density,
            self.cov0[0], self.cov0[1], self.cov0[2], 0.0,
            self.cov1[0], self.cov1[1], self.cov1[2], 0.0,
            self.cov2[0], self.cov2[1], self.cov2[2], 0.0,
            self.color[0], self.color[1], self.color[2], self.pad,
        ]
    }
}

fn splats_to_storage_bytes(splats: &[GpuSplatData]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(splats.len() * 20 * 4);

    for splat in splats {
        for value in splat.as_floats() {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
    }

    bytes
}

pub fn single_upward_segment_splat_bytes() -> Vec<u8> {
    let mut segments = Segments::new();
    let id = segments.push_root(Vector3::UP, 1.0);

    let direction = segments.direction(id).unwrap_or(Vector3::UP);
    let length = segments.length(id).unwrap_or(1.0);
    let splat = GpuSplatData::from_segment(direction, length);

    splats_to_storage_bytes(&[splat])
}
