use godot::builtin::Vector3;

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
    pub fn new(position: Vector3, color: [f32; 3], density: f32, sigma2: f32) -> Self {
        Self {
            position: [position.x, position.y, position.z],
            density,
            cov0: [sigma2, 0.0, 0.0],
            cov1: [0.0, sigma2, 0.0],
            cov2: [0.0, 0.0, sigma2],
            color,
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
    let mut splats = Vec::new();
    let steps = 27;
    let extent = 1.6;
    let sigma2 = 0.00005;
    let density = 0.07;

    for i in 0..steps {
        let t = (i as f32 / (steps - 1) as f32) * 2.0 - 1.0;
        let v = t * extent;

        splats.push(GpuSplatData::new(
            Vector3::new(v, 0.0, 0.0),
            [0.95, 0.25, 0.20],
            density,
            sigma2,
        ));
        splats.push(GpuSplatData::new(
            Vector3::new(0.0, v, 0.0),
            [0.20, 0.95, 0.30],
            density,
            sigma2,
        ));
        splats.push(GpuSplatData::new(
            Vector3::new(0.0, 0.0, v),
            [0.20, 0.45, 0.95],
            density,
            sigma2,
        ));
    }

    splats_to_storage_bytes(&splats)
}
