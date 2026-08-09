// entry point for the game / editor

use std::sync::Arc;

use cgmath::InnerSpace;
use cgmath::Rotation3;
use cgmath::Zero;
use piggies::camera;
use piggies::camera::Camera;
use piggies::instance::Instance;
use piggies::model;
use piggies::model::DrawModel;
use piggies::resource;
use piggies::texture;
use piggies::unlit_pipeline::create_unlit_pipeline;
use wgpu::util::DeviceExt;
use winit::event::MouseButton;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

// #[repr(C)]
// #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
// struct Vertex {
//     position: [f32; 3],
//     tex_coords: [f32; 2],
// }

// impl Vertex {
//     fn desc() -> wgpu::VertexBufferLayout<'static> {
//         wgpu::VertexBufferLayout {
//             array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
//             step_mode: wgpu::VertexStepMode::Vertex,
//             attributes: &[
//                 wgpu::VertexAttribute {
//                     offset: 0,
//                     shader_location: 0,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
//                     shader_location: 1,
//                     format: wgpu::VertexFormat::Float32x2,
//                 },
//             ],
//         }
//     }
// }

// const VERTICES: &[Vertex] = &[
//     // Changed
//     Vertex {
//         position: [-0.0868241, 0.49240386, 0.0],
//         tex_coords: [0.4131759, 0.00759614],
//     }, // A
//     Vertex {
//         position: [-0.49513406, 0.06958647, 0.0],
//         tex_coords: [0.0048659444, 0.43041354],
//     }, // B
//     Vertex {
//         position: [-0.21918549, -0.44939706, 0.0],
//         tex_coords: [0.28081453, 0.949397],
//     }, // C
//     Vertex {
//         position: [0.35966998, -0.3473291, 0.0],
//         tex_coords: [0.85967, 0.84732914],
//     }, // D
//     Vertex {
//         position: [0.44147372, 0.2347359, 0.0],
//         tex_coords: [0.9414737, 0.2652641],
//     }, // E
// ];

// const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

const NUM_INSTANCES_PER_ROW: u32 = 1;
const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
    0.0,
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
);

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    window: Arc<Window>,
    // vertex_buffer: wgpu::Buffer,
    // index_buffer: wgpu::Buffer,
    // num_indices: u32,
    camera: Camera,
    // camera_uniform: CameraUniform,
    // camera_buffer: wgpu::Buffer,
    // camera_bind_group: wgpu::BindGroup,
    camera_view_proj_state: camera::CameraRenderState,
    camera_inv_view_proj_state: camera::CameraRenderState,
    unlit_pipeline: wgpu::RenderPipeline,
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    depth_texture: texture::Texture,
    obj_model: model::Model,
    prev_time: std::time::Instant,
    w_is_down: bool,
    a_is_down: bool,
    s_is_down: bool,
    d_is_down: bool,
    q_is_down: bool,
    e_is_down: bool,
    mouse_left_down: bool,
    mouse_right_down: bool,
    prev_mouse_x: f32,
    prev_mouse_y: f32,
    mouse_x_delta: f32,
    mouse_y_delta: f32,
}

impl State {
    pub fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            apply_limit_buckets: true,
        });
        let adapter = pollster::block_on(adapter)?;
        let device_and_queue = adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
        });
        let (device, queue) = pollster::block_on(device_and_queue)?;
        let surface_caps = surface.get_capabilities((&adapter));
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        let camera = Camera {
            position: (0.0, 0.0, 5.0).into(),
            yaw_angle_deg: cgmath::Deg(0.0),
            pitch_angle_deg: cgmath::Deg(0.0),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let camera_view_proj_state = camera::CameraRenderState::new(
            &camera,
            &device,
            &queue,
            camera::CameraUniformType::ViewProjection,
        );
        let camera_inv_view_proj_state = camera::CameraRenderState::new(
            &camera,
            &device,
            &queue,
            camera::CameraUniformType::InverseViewProjNoTranslation,
        );
        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth texture");

        // let num_indices = INDICES.len() as u32;
        const SPACE_BETWEEN: f32 = 3.0;
        let instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let x = SPACE_BETWEEN * (x as f32 - (NUM_INSTANCES_PER_ROW - 1) as f32 / 2.0);
                    let z = SPACE_BETWEEN * (z as f32 - (NUM_INSTANCES_PER_ROW - 1) as f32 / 2.0);

                    let position = cgmath::Vector3 { x, y: 0.0, z };

                    let rotation = if position.is_zero() {
                        cgmath::Quaternion::from_axis_angle(
                            cgmath::Vector3::unit_z(),
                            cgmath::Deg(0.0),
                        )
                    } else {
                        cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
                    };

                    Instance { position, rotation }
                })
            })
            .collect::<Vec<_>>();

        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let unlit_pipeline = create_unlit_pipeline(&device, &config);
        let obj_model = resource::load_model_unlit("cube.obj", &device, &queue).unwrap();
        let prev_time = std::time::Instant::now();
        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
            camera,
            camera_view_proj_state,
            camera_inv_view_proj_state,
            unlit_pipeline,
            instance_buffer,
            instances,
            depth_texture,
            obj_model,
            prev_time,
            w_is_down: false,
            a_is_down: false,
            s_is_down: false,
            d_is_down: false,
            q_is_down: false,
            e_is_down: false,
            mouse_left_down: false,
            mouse_right_down: false,
            prev_mouse_x: 0.0,
            prev_mouse_y: 0.0,
            mouse_x_delta: 0.0,
            mouse_y_delta: 0.0,
        })
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
            self.depth_texture =
                texture::Texture::create_depth_texture(&self.device, &self.config, "depth texture");
        }
    }
    pub fn update(&mut self) -> anyhow::Result<()> {
        let now = std::time::Instant::now();
        let delta_time = (now - self.prev_time).as_secs_f32();
        self.prev_time = now;
        let speed = 1.0;
        if self.mouse_right_down {
            // handle movement
            if self.w_is_down {
                self.camera.move_forward(speed * delta_time)
            }
            if self.s_is_down {
                self.camera.move_forward(-speed * delta_time)
            }
            if self.a_is_down {
                self.camera.move_right(-speed * delta_time)
            }
            if self.d_is_down {
                self.camera.move_right(speed * delta_time)
            }
            if self.q_is_down {
                self.camera.move_up(-speed * delta_time)
            }
            if self.e_is_down {
                self.camera.move_up(speed * delta_time)
            }
            // handle mouse movement
            let x_delta = self.mouse_x_delta;
            let y_delta = self.mouse_y_delta;
            self.mouse_x_delta = 0.0;
            self.mouse_y_delta = 0.0;
            let yaw_rotation_speed = -0.1;
            let pitch_rotation_speed = -0.1;
            // let yaw_rotation_angle = yaw_rotation_speed * x_delta;
            // let pitch_rotation_angle = pitch_rotation_speed * y_delta;
            self.camera.yaw_angle_deg += cgmath::Deg(x_delta * yaw_rotation_speed);
            self.camera.pitch_angle_deg += cgmath::Deg(y_delta * pitch_rotation_speed);
        }
        Ok(())
    }
    pub fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();

        self.camera_view_proj_state
            .sync_camera_state(&self.camera, &self.queue);
        self.camera_inv_view_proj_state
            .sync_camera_state(&self.camera, &self.queue);

        if !self.is_surface_configured {
            return Ok(());
        }
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                anyhow::bail!("Lost device");
            }
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_pipeline(&self.unlit_pipeline);
            render_pass.draw_model_instanced(
                &self.obj_model,
                0..self.instances.len() as u32,
                &self.camera_view_proj_state.camera_bind_group,
            );
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        self.queue.present(output);
        Ok(())
    }
    fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        // match (code, is_pressed) {
        //     (KeyCode::Escape, true) => event_loop.exit(),
        //     (KeyCode::KeyW, true) => {
        //         self.w_is_down = true;
        //     }

        //     _ => {}
        // }
        match code {
            KeyCode::Escape => event_loop.exit(),
            KeyCode::KeyW => self.w_is_down = is_pressed,
            KeyCode::KeyA => self.a_is_down = is_pressed,
            KeyCode::KeyS => self.s_is_down = is_pressed,
            KeyCode::KeyD => self.d_is_down = is_pressed,
            KeyCode::KeyQ => self.q_is_down = is_pressed,
            KeyCode::KeyE => self.e_is_down = is_pressed,
            _ => {}
        }
    }
    fn handle_mouse_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        code: MouseButton,
        is_pressed: bool,
    ) {
        match code {
            MouseButton::Left => {
                self.mouse_left_down = is_pressed;
                println!("Mouse {}", if is_pressed { "down" } else { "up" })
            }
            MouseButton::Right => {
                self.mouse_right_down = is_pressed;
                println!("Mouse {}", if is_pressed { "down" } else { "up" })
            }
            _ => {}
        }
    }
    fn handle_mouse_move(&mut self, event_loop: &ActiveEventLoop, x: f32, y: f32) {
        self.mouse_x_delta = x - self.prev_mouse_x;
        self.mouse_y_delta = y - self.prev_mouse_y;
        self.prev_mouse_x = x;
        self.prev_mouse_y = y;
        println!(
            "Mouse move: ({}, {}), delta: ({}, {})",
            x, y, self.mouse_x_delta, self.mouse_y_delta
        );
    }
}

pub struct App {
    state: Option<State>,
}

impl App {
    pub fn new() -> Self {
        Self { state: None }
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes()
            .with_title("piggies")
            .with_inner_size(winit::dpi::PhysicalSize::new(1280, 720));
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.state = Some(State::new(window).unwrap());
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.camera.aspect = size.width as f32 / size.height as f32;

                state.resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
                state.update().unwrap();
                state.render().unwrap();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => {
                state.handle_key(event_loop, code, key_state.is_pressed());
            }
            WindowEvent::MouseInput {
                button,
                state: mouse_state,
                ..
            } => {
                state.handle_mouse_input(event_loop, button, mouse_state.is_pressed());
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                state.handle_mouse_move(event_loop, position.x as f32, position.y as f32);
            }
            _ => {}
        }
    }
    #[allow(unused_mut)]
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: State) {
        self.state = Some(event);
    }
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
}
pub fn run() -> anyhow::Result<()> {
    env_logger::init();
    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
fn main() -> anyhow::Result<()> {
    run()
}
