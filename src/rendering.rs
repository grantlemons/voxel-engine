pub struct Renderer {
    state: State,
    pub window: std::sync::Arc<winit::window::Window>,
    pub camera: Camera,
}
pub fn rotation_matrix(rad_rot: glam::Vec3) -> glam::Mat3 {
    glam::mat3(
        glam::vec3(
            rad_rot.y.cos() * rad_rot.z.cos(),
            rad_rot.y.cos() * rad_rot.z.sin(),
            -rad_rot.y.sin(),
        ),
        glam::vec3(
            rad_rot.x.sin() * rad_rot.y.sin() * rad_rot.z.cos() - rad_rot.x.cos() * rad_rot.z.sin(),
            rad_rot.x.sin() * rad_rot.y.sin() * rad_rot.z.sin() + rad_rot.x.cos() * rad_rot.z.cos(),
            rad_rot.x.sin() * rad_rot.y.cos(),
        ),
        glam::vec3(
            rad_rot.x.cos() * rad_rot.y.sin() * rad_rot.z.cos() + rad_rot.x.sin() * rad_rot.z.sin(),
            rad_rot.x.cos() * rad_rot.y.sin() * rad_rot.z.sin() - rad_rot.x.sin() * rad_rot.z.cos(),
            rad_rot.x.cos() * rad_rot.y.cos(),
        ),
    )
}

// 32 bytes
#[repr(C, align(16))]
#[derive(Default, Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera {
    pub rotation: [f32; 3],
    _padding_1: [u8; 4],
    pub position: [f32; 3],
    _padding_2: [u8; 4],
    pub size: [u32; 2],
    pub fov: f32,
    _padding: [u8; 4],
}

pub struct State {
    pub window: std::sync::Arc<winit::window::Window>,
    surface: wgpu::Surface<'static>,
    is_surface_configured: bool,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
}

impl State {
    pub async fn new(window: std::sync::Arc<winit::window::Window>) -> anyhow::Result<Self> {
        let num_push_vectors = std::mem::size_of::<[Camera; 1]>() as u32;
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: Default::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::PUSH_CONSTANTS,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits {
                    max_push_constant_size: num_push_vectors,
                    ..Default::default()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|fmt| fmt.is_srgb())
            .unwrap_or(&surface_caps.formats[0])
            .to_owned();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let shader = device.create_shader_module(wgpu::include_wgsl!("test_shader.wgsl"));

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Group Layout"),
            entries: &[],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Group"),
            layout: &bind_group_layout,
            entries: &[],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::FRAGMENT,
                range: 0..num_push_vectors,
            }],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        Ok(Self {
            device,
            window,
            surface,
            is_surface_configured: false,
            queue,
            config,
            pipeline,
            bind_group,
        })
    }
}

impl Renderer {
    pub async fn new(window: std::sync::Arc<winit::window::Window>) -> anyhow::Result<Self> {
        Ok(Self {
            window: window.clone(),
            state: State::new(window).await?,
            camera: Default::default(),
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.state.config.width = width;
            self.state.config.height = height;
            self.state
                .surface
                .configure(&self.state.device, &self.state.config);

            self.camera.size = [width, height];

            self.state.is_surface_configured = true;
        }
    }

    fn run_texture_shader(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        window_view: &wgpu::TextureView,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: window_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Discard,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.state.pipeline);
        render_pass.set_push_constants(
            wgpu::ShaderStages::FRAGMENT,
            0,
            bytemuck::bytes_of(&self.camera),
        );
        render_pass.set_bind_group(0, &self.state.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // We can't render unless the surface is configured
        if !self.state.is_surface_configured {
            return Ok(());
        }

        let window = self.state.surface.get_current_texture()?;
        let window_view = window.texture.create_view(&Default::default());
        let mut encoder =
            self.state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        self.run_texture_shader(&mut encoder, &window_view);

        self.state.queue.submit(std::iter::once(encoder.finish()));
        self.state.window.pre_present_notify();
        window.present();

        Ok(())
    }

    pub fn camera_left_right(&mut self, dist: f32) {
        let rad_rot = glam::vec3(
            self.camera.rotation[0].to_radians(),
            self.camera.rotation[1].to_radians(),
            self.camera.rotation[2].to_radians(),
        );
        let rot_mat = rotation_matrix(rad_rot);
        let right_dir = rot_mat * glam::vec3(1., 0., 0.);

        let position = glam::Vec3::from_slice(&self.camera.position);
        self.camera.position = (position + (dist * right_dir)).to_array();
        self.window.request_redraw();
    }
    pub fn camera_forward_back(&mut self, dist: f32) {
        let rad_rot = glam::vec3(
            self.camera.rotation[0].to_radians(),
            self.camera.rotation[1].to_radians(),
            self.camera.rotation[2].to_radians(),
        );
        let rot_mat = rotation_matrix(rad_rot);
        let forward_dir = rot_mat * glam::vec3(0., 0., 1.);

        let position = glam::Vec3::from_slice(&self.camera.position);
        self.camera.position = (position + (dist * forward_dir)).to_array();
        self.window.request_redraw();
    }

    pub fn rot_x(&mut self, dist: f32) {
        self.camera.rotation[0] += dist;
        self.camera.rotation[0] %= 360.;
        self.window.request_redraw();
    }
    pub fn rot_y(&mut self, dist: f32) {
        self.camera.rotation[1] += dist;
        self.camera.rotation[1] %= 360.;
        self.window.request_redraw();
    }
    pub fn rot_z(&mut self, dist: f32) {
        self.camera.rotation[2] += dist;
        self.camera.rotation[2] %= 360.;
        self.window.request_redraw();
    }
    pub fn reset_camera(&mut self) {
        self.camera.rotation = Default::default();
        self.camera.position = Default::default();
        self.camera.fov = 90.;
        self.window.request_redraw();
    }
}
