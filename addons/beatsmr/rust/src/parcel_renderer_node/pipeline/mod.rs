use godot::classes::{Camera3D, RenderingDevice};
use godot::prelude::*;

mod geometry;
mod resources;
mod tree;

use geometry::{camera_uniform_bytes, sample_parcels_for_time, PARCEL_FLOATS_PER_INSTANCE};
use super::ParcelRendererNode;

impl ParcelRendererNode {
    pub(super) fn parcel_buffer_size_bytes() -> u32 {
        (geometry::MAX_PARCELS * PARCEL_FLOATS_PER_INSTANCE * std::mem::size_of::<f32>()) as u32
    }

    pub(super) fn sample_parcel_buffer(time_seconds: f32) -> Vec<f32> {
        let parcels = sample_parcels_for_time(time_seconds);
        static BUFFER_UPLOAD_LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !BUFFER_UPLOAD_LOGGED.load(std::sync::atomic::Ordering::Relaxed) {
            godot_print!(
                "scene buffer upload: {} parcels (capacity: {})",
                parcels.len(),
                geometry::SCENE_PARCEL_COUNT,
            );
            BUFFER_UPLOAD_LOGGED.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let mut values = vec![0.0f32; geometry::SCENE_PARCEL_COUNT * PARCEL_FLOATS_PER_INSTANCE];

        for (index, parcel) in parcels.iter().enumerate() {
            let start = index * PARCEL_FLOATS_PER_INSTANCE;
            let packed = parcel.as_f32s();
            values[start..start + PARCEL_FLOATS_PER_INSTANCE].copy_from_slice(&packed);
        }

        values
    }

    pub(super) fn camera_data_for_frame(&mut self) -> PackedByteArray {
        if let Some(vp) = self.base().get_viewport() {
            if let Some(camera) = vp.get_camera_3d() {
                return camera_uniform_bytes(
                    camera.get_camera_transform(),
                    camera.get_camera_projection(),
                );
            }
        }

        if let Some(camera) = self
            .base()
            .try_get_node_as::<Camera3D>("../Camera3D")
            .or_else(|| self.base().try_get_node_as::<Camera3D>("Camera3D"))
        {
            return camera_uniform_bytes(
                camera.get_camera_transform(),
                camera.get_camera_projection(),
            );
        }

        camera_uniform_bytes(Transform3D::IDENTITY, Projection::IDENTITY)
    }

    pub(super) fn destroy_resources(&mut self, rd: &mut Gd<RenderingDevice>) {
        resources::destroy_resources(self, rd);
    }

    pub(super) fn recreate_resources(&mut self, rd: &mut Gd<RenderingDevice>) {
        resources::recreate_resources(self, rd);
    }

    pub(super) fn create_texture(&mut self, rd: &mut Gd<RenderingDevice>) {
        resources::create_texture(self, rd);
    }

    pub(super) fn create_pipeline_and_uniforms(&mut self, rd: &mut Gd<RenderingDevice>) {
        resources::create_pipeline_and_uniforms(self, rd);
    }
}
