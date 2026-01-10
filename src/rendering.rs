const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;

pub struct Renderer {
    state: State,
    pub window: std::sync::Arc<winit::window::Window>,
    pub camera: Camera,
}

// 32 bytes
#[repr(C, align(16))]
#[derive(Default, Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub fov: f32,
    padding: [u8; 4],
}

pub struct ComputeState {
    pipeline: wgpu::ComputePipeline,
    write_texture: Option<wgpu::Texture>,
    write_texture_view: Option<wgpu::TextureView>,
    bind_group: Option<wgpu::BindGroup>,
}

pub struct RenderState {
    pipeline: wgpu::RenderPipeline,
    read_texture: Option<wgpu::Texture>,
    read_texture_view: Option<wgpu::TextureView>,
    bind_group: Option<wgpu::BindGroup>,
}

pub struct State {
    pub window: std::sync::Arc<winit::window::Window>,
    surface: wgpu::Surface<'static>,
    is_surface_configured: bool,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    compute: ComputeState,
    render: RenderState,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl ComputeState {
    pub fn new(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        pipeline_layout: &wgpu::PipelineLayout,
    ) -> Self {
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(pipeline_layout),
            module: shader,
            entry_point: Some("cs_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            pipeline,
            write_texture: None,
            write_texture_view: None,
            bind_group: None,
        }
    }
}

impl RenderState {
    pub fn new(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        pipeline_layout: &wgpu::PipelineLayout,
        config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: shader,
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

        Self {
            pipeline,
            read_texture: None,
            read_texture_view: None,
            bind_group: None,
        }
    }
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
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: TEXTURE_FORMAT,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::ReadOnly,
                        format: TEXTURE_FORMAT,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::COMPUTE,
                range: 0..num_push_vectors,
            }],
        });

        Ok(Self {
            compute: ComputeState::new(&device, &shader, &pipeline_layout),
            render: RenderState::new(&device, &shader, &pipeline_layout, &config),
            device,
            window,
            surface,
            is_surface_configured: false,
            queue,
            config,
            bind_group_layout,
        })
    }

    pub fn update_textures(&mut self, width: u32, height: u32) {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let write_texture_descriptor = wgpu::TextureDescriptor {
            label: Some("Write Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        };
        let write_texture = self.device.create_texture(&write_texture_descriptor);
        let read_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Read Texture"),
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_DST,
            ..write_texture_descriptor
        });

        let write_texture_view = write_texture.create_view(&Default::default());
        let read_texture_view = read_texture.create_view(&Default::default());

        let compute_bind_group_descriptor = wgpu::BindGroupDescriptor {
            label: Some("Compute Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&write_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&read_texture_view),
                },
            ],
        };
        let render_bind_group_descriptor = wgpu::BindGroupDescriptor {
            label: Some("Render Group"),
            ..compute_bind_group_descriptor
        };

        self.compute.bind_group = Some(
            self.device
                .create_bind_group(&compute_bind_group_descriptor),
        );
        self.render.bind_group = Some(self.device.create_bind_group(&render_bind_group_descriptor));

        self.compute.write_texture = Some(write_texture);
        self.render.read_texture = Some(read_texture);

        self.compute.write_texture_view = Some(write_texture_view);
        self.render.read_texture_view = Some(read_texture_view);
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

            self.state.update_textures(width, height);

            self.state.is_surface_configured = true;
        }
    }

    fn run_compute_shader(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Render Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_bind_group(0, self.state.compute.bind_group.as_ref().unwrap(), &[]);
        compute_pass.set_pipeline(&self.state.compute.pipeline);
        compute_pass.set_push_constants(0, bytemuck::bytes_of(&self.camera));
        compute_pass.dispatch_workgroups(self.state.config.width, self.state.config.height, 1);
    }

    fn run_texture_shader(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        window_view: &wgpu::TextureView,
    ) {
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: self.state.compute.write_texture.as_ref().unwrap(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: self.state.render.read_texture.as_ref().unwrap(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            self.state.compute.write_texture.as_ref().unwrap().size(),
        );

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

        render_pass.set_pipeline(&self.state.render.pipeline);
        render_pass.set_bind_group(0, self.state.render.bind_group.as_ref().unwrap(), &[]);
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

        self.run_compute_shader(&mut encoder);
        self.run_texture_shader(&mut encoder, &window_view);

        self.state.queue.submit(std::iter::once(encoder.finish()));
        self.state.window.pre_present_notify();
        window.present();

        Ok(())
    }

    pub fn camera_x(&mut self, dist: f32) {
        self.camera.position[0] += dist;
        self.window.request_redraw();
    }
    pub fn camera_y(&mut self, dist: f32) {
        self.camera.position[1] += dist;
        self.window.request_redraw();
    }
    pub fn camera_z(&mut self, dist: f32) {
        self.camera.position[2] += dist;
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
}
