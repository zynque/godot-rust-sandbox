use godot::classes::rendering_device::{DataFormat, ShaderStage, TextureUsageBits, UniformType};
use godot::classes::{Camera3D, ProjectSettings, RdShaderFile, RdTextureFormat, RdTextureView, RdUniform, RenderingDevice};
use godot::prelude::*;

use super::{ParcelRendererNode, SHADER_PATH};

const MAX_PARCELS: usize = 100;
const PARCEL_FLOATS_PER_INSTANCE: usize = 36;

type GpuVec3 = [f32; 4];
type GpuMat3 = [f32; 12];

#[repr(C)]
#[derive(Clone, Copy)]
struct ParcelGpu {
    mean: GpuVec3,
    covariance: GpuMat3,
    inverse_covariance: GpuMat3,
    peak_density: f32,
    peak_padding: [f32; 3],
    color: GpuVec3,
}

#[derive(Clone, Copy)]
struct ParcelShape {
    forward: Vector3,
    normal: Vector3,
    axis_length: f32,
    width: f32,
    height: f32,
}

impl ParcelGpu {
    fn as_f32s(&self) -> [f32; PARCEL_FLOATS_PER_INSTANCE] {
        let mut out = [0.0; PARCEL_FLOATS_PER_INSTANCE];
        let mut cursor = 0;

        out[cursor..cursor + self.mean.len()].copy_from_slice(&self.mean);
        cursor += self.mean.len();
        out[cursor..cursor + self.covariance.len()].copy_from_slice(&self.covariance);
        cursor += self.covariance.len();
        out[cursor..cursor + self.inverse_covariance.len()]
            .copy_from_slice(&self.inverse_covariance);
        cursor += self.inverse_covariance.len();
        out[cursor] = self.peak_density;
        cursor += 1;
        out[cursor..cursor + self.peak_padding.len()].copy_from_slice(&self.peak_padding);
        cursor += self.peak_padding.len();
        out[cursor..cursor + self.color.len()].copy_from_slice(&self.color);
        out
    }
}

fn orthogonalize(v1: Vector3, v2: Vector3) -> Vector3 {
    let projection = v1 * v2.dot(v1);
    let result = v2 - projection;
    if result.length_squared() <= f32::EPSILON {
        return Vector3::UP;
    }
    result.normalized()
}

fn vector_component(v: Vector3, axis: usize) -> f32 {
    match axis {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

fn outer_product(u: Vector3, v: Vector3) -> [[f32; 3]; 3] {
    let mut matrix = [[0.0; 3]; 3];
    for row in 0..3 {
        for col in 0..3 {
            matrix[row][col] = vector_component(u, row) * vector_component(v, col);
        }
    }
    matrix
}

fn flatten_mat3(matrix: [[f32; 3]; 3]) -> GpuMat3 {
    let mut out = [0.0; 12];
    for col in 0..3 {
        for row in 0..3 {
            out[col * 4 + row] = matrix[row][col];
        }
    }
    out
}

fn covariance_from_shape(shape: ParcelShape) -> GpuMat3 {
    let forward = shape.forward.normalized();
    let normal = orthogonalize(forward, shape.normal.normalized());
    let right = forward.cross(normal).normalized();

    let l2 = shape.axis_length * shape.axis_length;
    let w2 = shape.width * shape.width;
    let t2 = shape.height * shape.height;

    let mut matrix = [[0.0; 3]; 3];
    for row in 0..3 {
        for col in 0..3 {
            let forward_term = outer_product(forward, forward)[row][col];
            let right_term = outer_product(right, right)[row][col];
            let normal_term = outer_product(normal, normal)[row][col];
            matrix[row][col] = l2 * forward_term + w2 * right_term + t2 * normal_term;
        }
    }

    flatten_mat3(matrix)
}

fn inverse_covariance_from_shape(shape: ParcelShape) -> GpuMat3 {
    let forward = shape.forward.normalized();
    let normal = orthogonalize(forward, shape.normal.normalized());
    let right = forward.cross(normal).normalized();

    let l2 = shape.axis_length * shape.axis_length;
    let w2 = shape.width * shape.width;
    let t2 = shape.height * shape.height;

    let mut matrix = [[0.0; 3]; 3];
    for row in 0..3 {
        for col in 0..3 {
            let forward_term = outer_product(forward, forward)[row][col];
            let right_term = outer_product(right, right)[row][col];
            let normal_term = outer_product(normal, normal)[row][col];
            let inv_l2 = if l2 > 0.0 { 1.0 / l2 } else { 0.0 };
            let inv_w2 = if w2 > 0.0 { 1.0 / w2 } else { 0.0 };
            let inv_t2 = if t2 > 0.0 { 1.0 / t2 } else { 0.0 };
            matrix[row][col] = inv_l2 * forward_term + inv_w2 * right_term + inv_t2 * normal_term;
        }
    }

    flatten_mat3(matrix)
}

fn transform3d_to_mat4_cols(transform: Transform3D) -> [f32; 16] {
    let cols = transform.basis.to_cols();
    let a = cols[0];
    let b = cols[1];
    let c = cols[2];
    let o = transform.origin;

    [
        a.x, a.y, a.z, 0.0,
        b.x, b.y, b.z, 0.0,
        c.x, c.y, c.z, 0.0,
        o.x, o.y, o.z, 1.0,
    ]
}

fn projection_to_mat4_cols(projection: Projection) -> [f32; 16] {
    let x = projection.cols[0];
    let y = projection.cols[1];
    let z = projection.cols[2];
    let w = projection.cols[3];

    [
        x.x, x.y, x.z, x.w,
        y.x, y.y, y.z, y.w,
        z.x, z.y, z.z, z.w,
        w.x, w.y, w.z, w.w,
    ]
}

fn camera_uniform_bytes(camera_transform: Transform3D, camera_projection: Projection) -> PackedByteArray {
    let view = camera_transform.affine_inverse();
    let view_floats = transform3d_to_mat4_cols(view);
    let projection_floats = projection_to_mat4_cols(camera_projection);

    let mut bytes = Vec::with_capacity(32 * std::mem::size_of::<f32>());
    for f in view_floats {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    for f in projection_floats {
        bytes.extend_from_slice(&f.to_le_bytes());
    }

    PackedByteArray::from(bytes.as_slice())
}

fn sample_parcels_for_time(time_seconds: f32) -> [ParcelGpu; 6] {
    let c = (0.8 * time_seconds).cos();

    fn make_parcel(mean: Vector3, shape: ParcelShape, peak_density: f32, color: Vector3) -> ParcelGpu {
        let covariance = covariance_from_shape(shape);
        let inverse_covariance = inverse_covariance_from_shape(shape);
        ParcelGpu {
            mean: [mean.x, mean.y, mean.z, 0.0],
            covariance,
            inverse_covariance,
            peak_density,
            peak_padding: [0.0, 0.0, 0.0],
            color: [color.x, color.y, color.z, 0.0],
        }
    }

    let shape_a = ParcelShape {
        forward: Vector3::BACK,
        normal: Vector3::UP,
        axis_length: 0.3,
        width: 0.3,
        height: 0.02,
    };
    let shape_b = ParcelShape {
        forward: Vector3::BACK,
        normal: Vector3::UP,
        axis_length: 0.3,
        width: 0.3,
        height: 0.02,
    };
    let shape_c = ParcelShape {
        forward: Vector3::BACK,
        normal: Vector3::UP,
        axis_length: 0.3,
        width: 0.02,
        height: 0.05,
    };

    let sample_parcels = [
        make_parcel(Vector3::new(-0.6 * c, 1.0, -3.0), shape_a, 0.8, Vector3::new(0.25, 0.75, 0.35)),
        make_parcel(Vector3::new(0.6 * c, 1.0, -3.0), shape_b, 0.8, Vector3::new(0.25, 0.75, 0.35)),
        make_parcel(Vector3::new(0.0, 1.0, -3.0), shape_c, 0.8, Vector3::new(0.25, 0.75, 0.35)),
    ];

    let axis_x = ParcelShape {
        forward: Vector3::RIGHT,
        normal: Vector3::UP,
        axis_length: 18.0,
        width: 0.1,
        height: 0.1,
    };
    let axis_y = ParcelShape {
        forward: Vector3::UP,
        normal: Vector3::BACK,
        axis_length: 18.0,
        width: 0.1,
        height: 0.1,
    };
    let axis_z = ParcelShape {
        forward: Vector3::BACK,
        normal: Vector3::UP,
        axis_length: 18.0,
        width: 0.1,
        height: 0.1,
    };

    [
        make_parcel(Vector3::new(-4.0,0.0,-5.0), axis_x, 1.0, Vector3::new(1.0, 0.0, 0.0)),
        make_parcel(Vector3::new(0.0,-4.0,-5.0), axis_y, 1.0, Vector3::new(0.0, 1.0, 0.0)),
        make_parcel(Vector3::new(0.0,0.0,-7.0), axis_z, 1.0, Vector3::new(0.0, 0.2, 1.0)),
        sample_parcels[0],
        sample_parcels[1],
        sample_parcels[2],
    ]
}

impl ParcelRendererNode {
    pub(super) fn parcel_buffer_size_bytes() -> u32 {
        (MAX_PARCELS * PARCEL_FLOATS_PER_INSTANCE * std::mem::size_of::<f32>()) as u32
    }

    pub(super) fn sample_parcel_buffer(time_seconds: f32) -> Vec<f32> {
        let parcels = sample_parcels_for_time(time_seconds);
        let mut values = vec![0.0f32; MAX_PARCELS * PARCEL_FLOATS_PER_INSTANCE];

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
        if self.uniform_set_rid != Rid::Invalid {
            rd.free_rid(self.uniform_set_rid);
            self.uniform_set_rid = Rid::Invalid;
        }
        if self.pipeline_rid != Rid::Invalid {
            rd.free_rid(self.pipeline_rid);
            self.pipeline_rid = Rid::Invalid;
        }
        if self.shader_rid != Rid::Invalid {
            rd.free_rid(self.shader_rid);
            self.shader_rid = Rid::Invalid;
        }
        if self.texture_rid != Rid::Invalid {
            rd.free_rid(self.texture_rid);
            self.texture_rid = Rid::Invalid;
        }
        if self.time_buffer_rid != Rid::Invalid {
            rd.free_rid(self.time_buffer_rid);
            self.time_buffer_rid = Rid::Invalid;
        }
        if self.camera_buffer_rid != Rid::Invalid {
            rd.free_rid(self.camera_buffer_rid);
            self.camera_buffer_rid = Rid::Invalid;
        }
        if self.parcel_buffer_rid != Rid::Invalid {
            rd.free_rid(self.parcel_buffer_rid);
            self.parcel_buffer_rid = Rid::Invalid;
        }
    }

    pub(super) fn recreate_resources(&mut self, rd: &mut Gd<RenderingDevice>) {
        self.destroy_resources(rd);
        self.create_texture(rd);
        self.create_pipeline_and_uniforms(rd);
    }

    pub(super) fn create_texture(&mut self, rd: &mut Gd<RenderingDevice>) {
        let usage = TextureUsageBits::STORAGE_BIT | TextureUsageBits::SAMPLING_BIT;
        let data_format = DataFormat::R32G32B32A32_SFLOAT;

        if !rd.texture_is_format_supported_for_usage(data_format, usage) {
            self.texture_rid = Rid::Invalid;
            godot_warn!(
                "ParcelRendererNode: R32G32B32A32_SFLOAT not supported for STORAGE|SAMPLING usage."
            );
            return;
        }

        let mut format = RdTextureFormat::new_gd();
        format.set_width(self.width);
        format.set_height(self.height);
        format.set_format(data_format);
        format.set_usage_bits(usage);

        let view = RdTextureView::new_gd();
        self.texture_rid = rd.texture_create(&format, &view);
        if self.texture_rid != Rid::Invalid {
            return;
        }

        self.texture_rid = Rid::Invalid;
        godot_warn!("ParcelRendererNode: failed to create RGBA32F output texture.");
    }

    fn create_time_buffer(&mut self, rd: &mut Gd<RenderingDevice>) {
        self.time_buffer_rid = rd.uniform_buffer_create(16);
        if self.time_buffer_rid == Rid::Invalid {
            godot_warn!("ParcelRendererNode: failed to create iTime uniform buffer.");
        }
    }

    fn create_camera_buffer(&mut self, rd: &mut Gd<RenderingDevice>) {
        self.camera_buffer_rid = rd.uniform_buffer_create(128);
        if self.camera_buffer_rid == Rid::Invalid {
            godot_warn!("ParcelRendererNode: failed to create camera uniform buffer.");
        }
    }

    fn create_parcel_buffer(&mut self, rd: &mut Gd<RenderingDevice>) {
        let byte_count = Self::parcel_buffer_size_bytes();
        let initial_data = PackedByteArray::from(vec![0u8; byte_count as usize].as_slice());
        self.parcel_buffer_rid = rd.storage_buffer_create_ex(byte_count).data(&initial_data).done();
        if self.parcel_buffer_rid == Rid::Invalid {
            godot_warn!("ParcelRendererNode: failed to create parcel storage buffer.");
        }
    }

    fn collapse_log_lines(value: &str) -> String {
        value
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" | ")
    }

    fn log_shader_source_attempt() {
        let file_path = ProjectSettings::singleton()
            .globalize_path(SHADER_PATH)
            .to_string();
        let source = std::fs::read_to_string(&file_path).unwrap_or_else(|error| {
            format!("failed to read shader source from '{file_path}': {error}")
        });

        let compact = Self::collapse_log_lines(&source);
        godot_warn!(
            "ParcelRendererNode: compiling shader '{}'. Source preview: {}",
            SHADER_PATH,
            compact
        );
    }

    pub(super) fn create_pipeline_and_uniforms(&mut self, rd: &mut Gd<RenderingDevice>) {
        if self.texture_rid == Rid::Invalid {
            godot_warn!("ParcelRendererNode: output texture RID is invalid.");
            return;
        }

        self.create_time_buffer(rd);
        if self.time_buffer_rid == Rid::Invalid {
            return;
        }

        self.create_camera_buffer(rd);
        if self.camera_buffer_rid == Rid::Invalid {
            return;
        }

        self.create_parcel_buffer(rd);
        if self.parcel_buffer_rid == Rid::Invalid {
            return;
        }

        let shader_file: Gd<RdShaderFile> = load(SHADER_PATH);
        if !shader_file.is_instance_valid() {
            godot_warn!("ParcelRendererNode: failed to load shader file {}", SHADER_PATH);
            return;
        }

        let Some(spirv) = shader_file.get_spirv() else {
            godot_warn!("ParcelRendererNode: failed to load SPIR-V from {}", SHADER_PATH);
            Self::log_shader_source_attempt();
            return;
        };

        let bytecode = spirv.get_stage_bytecode(ShaderStage::COMPUTE);
        let byte_count = bytecode.len();
        let compile_error = spirv.get_stage_compile_error(ShaderStage::COMPUTE);

        if !compile_error.is_empty() {
            let compact_error = Self::collapse_log_lines(&compile_error.to_string());
            godot_error!(
                "ParcelRendererNode: shader compile error for {} (compute stage): {}",
                SHADER_PATH,
                compact_error,
            );
        }

        godot_print!(
            "ParcelRendererNode: creating shader from SPIR-V bytes for {} ({} bytes).",
            SHADER_PATH,
            byte_count,
        );

        self.shader_rid = rd.shader_create_from_spirv(&spirv);
        if self.shader_rid == Rid::Invalid {
            godot_error!(
                "ParcelRendererNode: shader creation failed for {}. This usually means the bytecode or included shaders are invalid; shader source dump follows.",
                SHADER_PATH,
            );
            if !compile_error.is_empty() {
                let compact_error = Self::collapse_log_lines(&compile_error.to_string());
                godot_error!("ParcelRendererNode: shader compile details: {}", compact_error);
            }
            Self::log_shader_source_attempt();
            return;
        }

        self.pipeline_rid = rd.compute_pipeline_create(self.shader_rid);
        if self.pipeline_rid == Rid::Invalid {
            godot_warn!("ParcelRendererNode: failed to create compute pipeline RID.");
            return;
        }

        let mut output_image = RdUniform::new_gd();
        output_image.set_uniform_type(UniformType::IMAGE);
        output_image.set_binding(0);
        output_image.add_id(self.texture_rid);

        let mut time_uniform = RdUniform::new_gd();
        time_uniform.set_uniform_type(UniformType::UNIFORM_BUFFER);
        time_uniform.set_binding(1);
        time_uniform.add_id(self.time_buffer_rid);

        let mut camera_uniform = RdUniform::new_gd();
        camera_uniform.set_uniform_type(UniformType::UNIFORM_BUFFER);
        camera_uniform.set_binding(2);
        camera_uniform.add_id(self.camera_buffer_rid);

        let mut parcel_uniform = RdUniform::new_gd();
        parcel_uniform.set_uniform_type(UniformType::STORAGE_BUFFER);
        parcel_uniform.set_binding(3);
        parcel_uniform.add_id(self.parcel_buffer_rid);

        self.uniform_set_rid = rd.uniform_set_create(
            &Array::from_iter([output_image, time_uniform, camera_uniform, parcel_uniform]),
            self.shader_rid,
            0,
        );
        if self.uniform_set_rid == Rid::Invalid {
            godot_warn!(
                "ParcelRendererNode: failed to create uniform set (set=0, binding=0 image, binding=1 iTime, binding=2 camera, binding=3 parcels)."
            );
        }
    }
}
