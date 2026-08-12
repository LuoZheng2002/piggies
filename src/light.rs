use wgpu::util::DeviceExt;

pub enum SingleLight {
    Point { position: cgmath::Point3<f32> },
    Directional { direction: cgmath::Vector3<f32> },
}
pub struct Lights(pub Vec<SingleLight>);

pub struct LightRenderState {
    pub light_uniform: LightUniform,
    pub light_buffer: wgpu::Buffer,
    pub light_bind_group: wgpu::BindGroup,
}

impl LightRenderState {
    pub fn new(lights: &Lights, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let light_uniform = LightUniform::from_lights(lights);
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &Self::light_bind_group_layout(device),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
            label: Some("light_bind_group"),
        });
        let mut output = Self {
            light_uniform,
            light_bind_group,
            light_buffer,
        };
        output.sync_lights(lights, queue);
        output
    }
    pub fn sync_lights(&mut self, lights: &Lights, queue: &wgpu::Queue) {
        self.light_uniform = LightUniform::from_lights(lights);
        queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::cast_slice(&[self.light_uniform]),
        );
    }
    pub fn light_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        todo!()
    }
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SingleLightUniform {
    pub position: [f32; 4],
    pub color: [f32; 4],
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub num_lights: u32,
    pub _padding: u32,
    pub lights: [SingleLightUniform; 16],
}

impl LightUniform {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn from_lights(lights: &Lights) -> LightUniform {
        let num_lights = lights.0.len() as _;
        let mut lights_uniform: [SingleLightUniform; 16] = [Default::default(); 16];
        for (i, light) in lights.0.iter().enumerate() {
            lights_uniform[i].position = match light {
                SingleLight::Point { position } => [position.x, position.y, position.z, 1.0],
                SingleLight::Directional { direction } => {
                    [direction.x, direction.y, direction.z, 0.0]
                }
            }
        }
        Self {
            num_lights,
            _padding: 0,
            lights: lights_uniform,
        }
    }
}
