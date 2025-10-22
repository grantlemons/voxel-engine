#![allow(dead_code, unused_imports)]

use std::sync::Arc;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Voxel {
    location: [f32; 3],
    dims: [f32; 3],
    color: [f32; 3],
}
const TEST_VOXELS: &[Voxel] = &[Voxel {
    location: [0., 0., 0.],
    dims: [1., 1., 1.],
    color: [255., 255., 255.],
}];
const TEST_LIGHTS: &[Voxel] = &[Voxel {
    location: [-4., -4., 4.],
    dims: [1., 1., 1.],
    color: [255., 255., 255.],
}];

use bitflags::Flags;
use wgpu::{
    Device, ExperimentalFeatures, MemoryHints, PipelineCompilationOptions, Queue, RenderPipeline,
    Surface, SurfaceConfiguration, util::DeviceExt,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

pub struct State {
    window: Arc<Window>,
    size: PhysicalSize<u32>,
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    render_pipeline: RenderPipeline,
    frame_count: u32,
    voxel_buffer: wgpu::Buffer,
    lights_buffer: wgpu::Buffer,
}

impl State {
    pub async fn new(window: Window) -> Self {
        let size = window.inner_size();
        let window = Arc::new(window);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
        let surface = instance
            .create_surface(Arc::clone(&window))
            .expect("Unable to create surface!");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::None,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Unable to create GPU adapter!");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::PUSH_CONSTANTS,
                required_limits: wgpu::Limits {
                    max_push_constant_size: 256,
                    ..Default::default()
                },
                experimental_features: ExperimentalFeatures::default(),
                memory_hints: MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Unable to create GPU device!");
        let capabilities = surface.get_capabilities(&adapter);
        let surface_format = capabilities
            .formats
            .iter()
            .find(|fmt| fmt.is_srgb())
            .unwrap_or(&capabilities.formats[0])
            .to_owned();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: capabilities.present_modes[0],
            desired_maximum_frame_latency: 2,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: Vec::new(),
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Cubic Raymarching"),
            source: wgpu::ShaderSource::Wgsl(include_str!("raymarching.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Voxel and Light list layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                range: 0..std::mem::size_of::<[f32; 1]>() as u32, // parameters
            }],
        });

        let voxel_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Voxel Buffer"),
            contents: bytemuck::cast_slice(TEST_VOXELS),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let lights_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lights Buffer"),
            contents: bytemuck::cast_slice(TEST_LIGHTS),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipleline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
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

        Self {
            window,
            size,
            surface,
            device,
            queue,
            config,
            render_pipeline,
            frame_count: 0,
            voxel_buffer,
            lights_buffer,
        }
    }
}
