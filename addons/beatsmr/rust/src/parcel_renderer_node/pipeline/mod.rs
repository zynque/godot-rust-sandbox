use godot::classes::{Camera3D, RenderingDevice};
use godot::prelude::*;

mod camera;
mod parcels;
mod resources;

use camera::camera_uniform_bytes;
use super::ParcelRendererNode;

pub(crate) use parcels::{parcel_bytes, MAX_PARCELS, PARCEL_FLOATS_PER_INSTANCE};

impl ParcelRendererNode {
    pub(super) fn parcel_buffer_size_bytes() -> u32 {
        (MAX_PARCELS * PARCEL_FLOATS_PER_INSTANCE * std::mem::size_of::<f32>()) as u32
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
