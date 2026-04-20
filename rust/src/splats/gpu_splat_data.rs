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
