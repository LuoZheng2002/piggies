use wgpu::util::DeviceExt;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
    cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);

pub struct Camera {
    pub position: cgmath::Point3<f32>,
    // forward: cgmath::Vector3<f32>,
    // up: cgmath::Vector3<f32>,

    // we do not include roll in the camera rotation for now
    pub yaw_angle_deg: cgmath::Deg<f32>,
    pub pitch_angle_deg: cgmath::Deg<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        // let view = cgmath::Matrix4::look_at_rh(self.eye, self.eye + self.forward, self.up);
        // we build the view from the camera's yaw and pitch angles
        // // use cgmath::Matrix4::from_angle_x, from_angle_y, from_angle_z
        let view = cgmath::Matrix4::look_at_rh(
            self.position,
            self.position + self.forward(),
            cgmath::Vector3::new(0.0, 1.0, 0.0),
        );

        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        OPENGL_TO_WGPU_MATRIX * proj * view
    }

    pub fn forward(&self) -> cgmath::Vector3<f32> {
        // consider the pitch and yaw angles to compute the forward direction
        let initial_forward = cgmath::Vector4::new(0.0, 0.0, -1.0, 0.0);
        let forward = cgmath::Matrix4::from_angle_x(self.pitch_angle_deg) * initial_forward;
        let forward = cgmath::Matrix4::from_angle_y(self.yaw_angle_deg) * forward;
        cgmath::Vector3::new(forward.x, forward.y, forward.z)
    }

    pub fn move_forward(&mut self, delta: f32) {
        self.position += self.forward() * delta;
    }

    pub fn move_backward(&mut self, delta: f32) {
        self.position -= self.forward() * delta;
    }

    pub fn move_left(&mut self, delta: f32) {
        self.position -= self.right() * delta;
    }

    pub fn move_right(&mut self, delta: f32) {
        self.position += self.right() * delta;
    }

    pub fn move_up(&mut self, delta: f32) {
        // should be perpendicular to the forward direction
        // right cross forward
        let up = self.right().cross(self.forward());
        self.position += up * delta;
    }

    pub fn right(&self) -> cgmath::Vector3<f32> {
        let up = cgmath::Vector3::new(0.0, 1.0, 0.0);
        self.forward().cross(up)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    transform: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            transform: cgmath::Matrix4::identity().into(),
        }
    }

    // pub fn update(&mut self, camera_uniform_type camera: &Camera) {
    //     match self.camera_uniform_type {
    //         CameraUniformType::ViewProjection => self.view_proj = camera.build_view_projection_matrix().into(),
    //         CameraUniformType::InverseViewProjNoTranslation => self.view_proj = camera.build_inverse_view_proj_no_translation_matrix().into(),
    //     }
    // }
    pub fn update_view_proj(&mut self, camera: &Camera) {
        let view = cgmath::Matrix4::look_at_rh(
            camera.position,
            camera.position + camera.forward(),
            cgmath::Vector3::new(0.0, 1.0, 0.0),
        );

        let proj = cgmath::perspective(
            cgmath::Deg(camera.fovy),
            camera.aspect,
            camera.znear,
            camera.zfar,
        );
        self.transform = (OPENGL_TO_WGPU_MATRIX * proj * view).into();
    }
    pub fn update_inverse_view_proj_no_translation(&mut self, camera: &Camera) {
        use cgmath::SquareMatrix;

        let origin = cgmath::Point3::new(0.0, 0.0, 0.0);
        let view = cgmath::Matrix4::look_at_rh(
            origin,
            origin + camera.forward(),
            cgmath::Vector3::new(0.0, 1.0, 0.0),
        );

        let proj = cgmath::perspective(
            cgmath::Deg(camera.fovy),
            camera.aspect,
            camera.znear,
            camera.zfar,
        );
        self.transform = (OPENGL_TO_WGPU_MATRIX * proj * view)
            .invert()
            .expect("view-projection matrix should be invertible")
            .into();
    }
}

pub enum CameraUniformType {
    ViewProjection,
    InverseViewProjNoTranslation,
}
pub struct CameraRenderState {
    pub camera_uniform_type: CameraUniformType,
    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
}

impl CameraRenderState {
    pub fn new(
        camera: &Camera,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera_uniform_type: CameraUniformType,
    ) -> Self {
        let camera_uniform = CameraUniform::new();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &Self::camera_bind_group_layout(device),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });
        let mut output = Self {
            camera_uniform_type,
            camera_uniform,
            camera_bind_group,
            camera_buffer,
        };
        output.sync_camera_state(camera, queue);
        output
    }
    pub fn sync_camera_state(&mut self, camera: &Camera, queue: &wgpu::Queue) {
        match self.camera_uniform_type {
            CameraUniformType::ViewProjection => {
                self.camera_uniform.update_view_proj(camera);
            }
            CameraUniformType::InverseViewProjNoTranslation => {
                self.camera_uniform
                    .update_inverse_view_proj_no_translation(camera);
            }
        }
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }
    pub fn camera_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });
        camera_bind_group_layout
    }
}
