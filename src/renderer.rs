use crate::geometry::{SphereGeometry, SphereVertex};
use crate::{shader_module::ShaderModuleBuilder, texture};
use std::sync::Arc;
use texture::Texture;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, Event as WinitEvent, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

pub struct Renderer {
    surface: Option<wgpu::Surface<'static>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    config: Option<wgpu::SurfaceConfiguration>,
    size: Option<winit::dpi::PhysicalSize<u32>>,
    window: Option<Arc<Window>>,

    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    num_indices: Option<u32>,
    render_pipeline: Option<wgpu::RenderPipeline>,
    depth_texture: Option<texture::Texture>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            surface: None,
            device: None,
            queue: None,
            config: None,
            size: None,
            window: None,

            vertex_buffer: None,
            index_buffer: None,
            num_indices: None,
            render_pipeline: None,
            depth_texture: None,
        }
    }
}

impl ApplicationHandler for Renderer {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = WindowAttributes::default()
                .with_title("MLS MPM Render")
                .with_inner_size(winit::dpi::LogicalSize::new(800, 600));

            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

            // Initialize wgpu
            pollster::block_on(self.init_renderer(window.clone()));
            self.window = Some(window);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(window) = &self.window {
            if window_id == window.id() {
                // self.input(&event);
                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::Escape),
                                ..
                            },
                        ..
                    } => event_loop.exit(),

                    WindowEvent::Resized(physical_size) => {
                        self.resize(physical_size);
                    }

                    WindowEvent::RedrawRequested => {
                        // self.update();
                        match self.render() {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => {
                                // let size = window.inner_size()
                                // self.resize(size);
                            }
                            Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                            Err(e) => eprintln!("Render error: {:?}", e),
                        }
                    }

                    _ => {}
                }
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl Renderer {
    async fn init_renderer(&mut self, window: Arc<Window>) {
        let size: PhysicalSize<u32> = window.inner_size();
        // Create instance surface adpater device and queue
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).unwrap();
        // request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        // request device and queue
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: Some("Device"),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();
        // configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let sphere = SphereGeometry::default_sphere(0.1);
        let render_data = sphere.create_render_data(&device);
        let vertex_buffer = render_data.vertex_buffer;
        let index_buffer = render_data.index_buffer;
        let num_indices = render_data.num_indices;

        // setup depth texture
        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "Depth Texture");

        // create render pipeline
        let vertex_shader = ShaderModuleBuilder::new()
            .add_module(include_str!("./vertex_shader.wgsl"))
            .build(&device, Some("Vertex Shader"));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: Some("vs_main"),
                buffers: &[SphereVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &vertex_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Set fields
        self.surface = Some(surface);
        self.device = Some(device);
        self.queue = Some(queue);
        self.config = Some(config);
        self.size = Some(size);
        self.window = Some(window);

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
        self.num_indices = Some(num_indices);
        self.render_pipeline = Some(render_pipeline);
        self.depth_texture = Some(depth_texture);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            if let (Some(config), Some(surface), Some(device)) =
                (&mut self.config, &self.surface, &self.device)
            {
                config.width = new_size.width;
                config.height = new_size.height;
                self.depth_texture = Some(texture::Texture::create_depth_texture(
                    &device,
                    &config,
                    "depth texture",
                ));
                surface.configure(device, config);
            }
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        todo!()
    }

    fn update(&mut self) {
        todo!()
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Get references to all needed components
        let (surface, device, queue, render_pipeline) = match (
            &self.surface,
            &self.device,
            &self.queue,
            &self.render_pipeline,
        ) {
            (Some(surface), Some(device), Some(queue), Some(pipeline)) => {
                (surface, device, queue, pipeline)
            }
            _ => return Ok(()), // Not initialized yet
        };
        let output = surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        // command encoder
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
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
                    view: &self
                        .depth_texture
                        .as_ref()
                        .expect("depth text not init")
                        .view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Set pipeline and draw
            render_pass.set_pipeline(render_pipeline);
            render_pass.set_vertex_buffer(
                0,
                self.vertex_buffer
                    .as_ref()
                    .expect("Vertex buffer not init")
                    .slice(..),
            );
            render_pass.set_index_buffer(
                self.index_buffer
                    .as_ref()
                    .expect("Index buffer not init")
                    .slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..self.num_indices.expect("No vertex indices"), 0, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
