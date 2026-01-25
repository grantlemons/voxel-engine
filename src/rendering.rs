use std::sync::{Arc, LazyLock};

use glam::{Mat4, Vec3, Vec4Swizzles, vec4};
use wgpu::util::DeviceExt;

pub static VOXELS: LazyLock<[Voxel; 3]> = LazyLock::new(|| {
    [
        Voxel {
            position: [0., 0., 2.],
            color: [1., 1., 1.],
            ..Default::default()
        },
        Voxel {
            position: [1., 0., 3.],
            color: [1., 1., 1.],
            ..Default::default()
        },
        Voxel {
            position: [0., 1., 3.],
            color: [1., 1., 1.],
            ..Default::default()
        },
    ]
});

pub static LIGHTS: LazyLock<[Voxel; 2]> = LazyLock::new(|| {
    [
        Voxel {
            position: [2., 3., 0.],
            color: [255. / 255., 237. / 255., 222. / 255.],
            ..Default::default()
        },
        Voxel {
            position: [2., -3., 3.],
            color: [255. / 255., 237. / 255., 222. / 255.],
            ..Default::default()
        },
    ]
});

pub struct BufferWriteCommand {
    pub target_buffer: wgpu::Buffer,
    pub offset: u64,
    pub new_data: Vec<u8>,
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera {
    pub rotation_matrix: [f32; 16],
    pub position: [f32; 3],
    _padding_1: u32,
    pub size: [u32; 2],
    pub fov: f32,
    _padding_2: u32,
}

#[repr(C, align(16))]
#[derive(Debug, Default, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Voxel {
    pub position: [f32; 3],
    pub _padding_1: u32,
    pub color: [f32; 3],
    pub _padding_2: u32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            rotation_matrix: Mat4::default().to_cols_array(),
            position: Default::default(),
            size: Default::default(),
            fov: 60.,
            _padding_1: Default::default(),
            _padding_2: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct Renderer {
    state: State,
    pub window: Arc<winit::window::Window>,
    pub camera: Camera,
    pub buffers: Arc<Buffers>,
    pub buffer_writer: flume::Sender<BufferWriteCommand>,
    buffer_reader: flume::Receiver<BufferWriteCommand>,
}

#[derive(Debug)]
pub struct Buffers {
    pub voxels: wgpu::Buffer,
    pub lights: wgpu::Buffer,
}

#[derive(Debug)]
pub struct State {
    pub window: Arc<winit::window::Window>,
    surface: wgpu::Surface<'static>,
    is_surface_configured: bool,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    buffers: Arc<Buffers>,
}

impl State {
    pub async fn new(window: Arc<winit::window::Window>) -> anyhow::Result<Self> {
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
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let voxels = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Voxel List"),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::bytes_of(&*VOXELS),
        });

        let lights = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light List"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: bytemuck::bytes_of(&*LIGHTS),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &voxels,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &lights,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
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
            buffers: Arc::new(Buffers { voxels, lights }),
        })
    }
}

impl Renderer {
    pub async fn new(window: Arc<winit::window::Window>) -> anyhow::Result<Self> {
        let state = State::new(window.clone()).await?;
        let (buffer_writer, buffer_reader) = flume::unbounded();
        Ok(Self {
            window,
            buffers: state.buffers.clone(),
            state,
            camera: Default::default(),
            buffer_writer,
            buffer_reader,
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

        let mut belt = wgpu::util::StagingBelt::new(100);
        for command in self.buffer_reader.try_iter() {
            let mut view = belt.write_buffer(
                &mut encoder,
                &command.target_buffer,
                command.offset,
                std::num::NonZero::new(command.new_data.len() as u64).unwrap(),
                &self.state.device,
            );
            view.copy_from_slice(&command.new_data);
        }
        belt.finish();

        self.run_texture_shader(&mut encoder, &window_view);
        self.state.queue.submit(std::iter::once(encoder.finish()));

        belt.recall();

        self.state.window.pre_present_notify();
        window.present();

        Ok(())
    }

    pub fn camera_left_right(&mut self, dist: f32) {
        let right_dir = Mat4::from_cols_array(&self.camera.rotation_matrix) * vec4(1., 0., 0., 0.);

        let position = Vec3::from_slice(&self.camera.position);
        self.camera.position = (position + (dist * right_dir.xyz())).to_array();
        self.window.request_redraw();
    }
    pub fn camera_forward_back(&mut self, dist: f32) {
        let right_dir = Mat4::from_cols_array(&self.camera.rotation_matrix) * vec4(0., 0., 1., 0.);

        let position = Vec3::from_slice(&self.camera.position);
        self.camera.position = (position + (dist * right_dir.xyz())).to_array();
        self.window.request_redraw();
    }

    pub fn rot_x(&mut self, dist: f32) {
        let rot_mat = Mat4::from_cols_array(&self.camera.rotation_matrix)
            * Mat4::from_rotation_x((dist % 360.).to_radians());
        self.camera.rotation_matrix = rot_mat.to_cols_array();
        self.window.request_redraw();
    }
    pub fn rot_y(&mut self, dist: f32) {
        let rot_mat = Mat4::from_cols_array(&self.camera.rotation_matrix)
            * Mat4::from_rotation_y((dist % 360.).to_radians());
        self.camera.rotation_matrix = rot_mat.to_cols_array();
        self.window.request_redraw();
    }
    pub fn rot_z(&mut self, dist: f32) {
        let rot_mat = Mat4::from_cols_array(&self.camera.rotation_matrix)
            * Mat4::from_rotation_z((dist % 360.).to_radians());
        self.camera.rotation_matrix = rot_mat.to_cols_array();
        self.window.request_redraw();
    }
    pub fn reset_camera(&mut self) {
        self.camera = Camera {
            size: self.camera.size,
            ..Default::default()
        };
        self.window.request_redraw();
    }
}
