use crate::camera;
use crate::geometry::{SphereGeometry, SphereVertex};
// use crate::mls_mpm::{MlsMpm, MlsMpmCompute};
use crate::sph::{Sph, SphCompute};
use crate::{shader_module::ShaderModuleBuilder, texture};
use std::sync::Arc;
use texture::Texture;
use wgpu::ShaderStages;
use wgpu::util::DeviceExt;
use winit::event::DeviceEvent;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, Event as WinitEvent, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

use crate::CSVWriter::{write_particles_to_csv, write_motion_to_csv};

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct CameraUniform {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
}
impl CameraUniform {
    fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_position: [0.0; 4],
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }
    fn update_view_proj(&mut self, camera: &camera::Camera, projection: &camera::Projection) {
        self.view_position = camera.position.to_homogeneous().into();
        self.view_proj = (projection.calc_matrix() * camera.calc_matrix()).into();
    }
}

struct Instance {
    position: [f32; 3],
    _padding: f32,
    color: [f32; 4],
}
impl Instance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Instance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct Renderer {
    sim: Option<Sph>,
    compute: Option<SphCompute>,
    surface: Option<wgpu::Surface<'static>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    config: Option<wgpu::SurfaceConfiguration>,
    size: Option<winit::dpi::PhysicalSize<u32>>,
    window: Option<Arc<Window>>,

    camera: Option<camera::Camera>,
    projection: Option<camera::Projection>,
    camera_controller: Option<camera::CameraController>,
    camera_uniform: Option<CameraUniform>,
    camera_buffer: Option<wgpu::Buffer>,
    camera_bind_group: Option<wgpu::BindGroup>,
    mouse_pressed: bool,
    last_render_time: Option<instant::Instant>,

    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    num_indices: Option<u32>,
    render_pipeline: Option<wgpu::RenderPipeline>,
    depth_texture: Option<texture::Texture>,
    // Particle Instances, only need position, no rotation
    instance_buffer: Option<wgpu::Buffer>,
    particle_instance_bind_group: Option<wgpu::BindGroup>,
    particle_instance_pipeline: Option<wgpu::ComputePipeline>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            sim: None,
            compute: None,

            surface: None,
            device: None,
            queue: None,
            config: None,
            size: None,
            window: None,

            camera: None,
            projection: None,
            camera_controller: None,
            camera_uniform: None,
            camera_buffer: None,
            camera_bind_group: None,
            mouse_pressed: false,
            last_render_time: None,

            vertex_buffer: None,
            index_buffer: None,
            num_indices: None,
            render_pipeline: None,
            depth_texture: None,

            instance_buffer: None,
            particle_instance_bind_group: None,
            particle_instance_pipeline: None,
        }
    }
}

impl ApplicationHandler for Renderer {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = WindowAttributes::default()
                .with_title("Hydrocode Render")
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
                self.input(&event);
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
                        let now = instant::Instant::now();
                        let dt = if let Some(last_time) = self.last_render_time {
                            now.duration_since(last_time)
                        } else {
                            instant::Duration::from_millis(16)
                        };
                        self.last_render_time = Some(now);
                        self.update(dt);
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

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let Some(camera_controller) = &mut self.camera_controller {
            match event {
                DeviceEvent::MouseMotion { delta } => {
                    if self.mouse_pressed {
                        camera_controller.process_mouse(delta.0, delta.1);
                    }
                }
                _ => {}
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
    pub fn attach_sim(&mut self, sim: Sph) {
        self.sim = Some(sim);
        println!("Sim attached successfully");
    }

    async fn init_renderer(&mut self, window: Arc<Window>) {
        println!("init_renderer called");
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

        // Initialize Compute Buffers and Pipelines
        let sim = self.sim.as_ref().expect("Sim not initialized");
        let compute = pollster::block_on(SphCompute::new(&device, &sim.params));
        // write buffers to compute
        compute.cpu2gpu_params(&queue, &sim.params);
        //compute.cpu2gpu_disturbance(&queue, &sim.disturbance);
        compute.cpu2gpu_particles(&queue, &sim.particles, &sim.motion);
        compute.cpu2gpu_materials(&queue, &sim.materials);

        // Initialize Camera
        let camera = camera::Camera::new((0.0, 0.0, 2.5), cgmath::Deg(-90.0), cgmath::Deg(0.0));
        let projection =
            camera::Projection::new(config.width, config.height, cgmath::Deg(45.0), 0.1, 300.0);
        let camera_controller = camera::CameraController::new(1.0, 1.0);
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera, &projection);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

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
                label: Some("Camera Bind Group Layout"),
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("Camera Bind Group"),
        });

        let sphere = SphereGeometry::default_sphere(0.005);
        let render_data = sphere.create_render_data(&device);
        let vertex_buffer = render_data.vertex_buffer;
        let index_buffer = render_data.index_buffer;
        let num_indices = render_data.num_indices;

        // setup depth texture
        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "Depth Texture");

        // create modules
        let vertex_shader = ShaderModuleBuilder::new()
            .add_module(include_str!("./vertex_shader.wgsl"))
            .build(&device, Some("Vertex Shader"));
        let particle_to_instance_shader = ShaderModuleBuilder::new()
            .add_module(include_str!("./particle_to_instance.wgsl"))
            .build(&device, Some("Particle to Instance Shader"));

        let num_particles = sim.params.num_particles;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
            size: (std::mem::size_of::<Instance>() * num_particles as usize) as u64,
            mapped_at_creation: false,
        });

        let particle_instance_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("Particle Instance Bind Group Layout"),
            });

        let particle_instance_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &particle_instance_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: compute.buffer_particles.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: compute.buffer_materials.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: compute.buffer_params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: instance_buffer.as_entire_binding(),
                },
            ],
            label: Some("Particle Instance Bind Group"),
        });

        let particle_instance_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Particle Instance Pipeline Layout"),
                bind_group_layouts: &[&particle_instance_bind_group_layout],
                push_constant_ranges: &[],
            });
        let particle_instance_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Particle Instance Pipeline"),
                layout: Some(&particle_instance_pipeline_layout),
                module: &particle_to_instance_shader,
                entry_point: Some("particle_to_instance"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: Some("vs_main"),
                buffers: &[SphereVertex::desc(), Instance::desc()],
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
        self.compute = Some(compute);
        self.surface = Some(surface);
        self.device = Some(device);
        self.queue = Some(queue);
        self.config = Some(config);
        self.size = Some(size);
        self.window = Some(window);

        self.camera = Some(camera);
        self.projection = Some(projection);
        self.camera_controller = Some(camera_controller);
        self.camera_uniform = Some(camera_uniform);
        self.camera_buffer = Some(camera_buffer);
        self.camera_bind_group = Some(camera_bind_group);

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
        self.num_indices = Some(num_indices);
        self.render_pipeline = Some(render_pipeline);
        self.depth_texture = Some(depth_texture);

        self.instance_buffer = Some(instance_buffer);
        self.particle_instance_bind_group = Some(particle_instance_bind_group);
        self.particle_instance_pipeline = Some(particle_instance_pipeline);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            if let (Some(config), Some(surface), Some(device), Some(projection)) = (
                &mut self.config,
                &self.surface,
                &self.device,
                &mut self.projection,
            ) {
                config.width = new_size.width;
                config.height = new_size.height;
                projection.resize(new_size.width, new_size.height);
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
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                // Pass the entire KeyEvent, not just the KeyCode
                let key: &KeyEvent = event;
                if let Some(camera_controller) = &mut self.camera_controller {
                    camera_controller.process_keyboard(key);
                } else {
                }
                let physical_key = key.physical_key;
                let key_state = key.state;
                match physical_key {
                    PhysicalKey::Code(KeyCode::KeyR) => {
                        if let (Some(compute), Some(sim), Some(queue)) =
                            (&self.compute, &mut self.sim, &self.queue)
                        {
                            compute.cpu2gpu_particles(queue, &sim.particles, &sim.motion);
                            sim.sim_time = 0.0;
                            sim.sim_idx = 0;
                            true
                        } else {
                            false
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyT) => {
                        if let (Some(sim),) =
                            (&self.sim,)
                        {
                            if key_state == ElementState::Released {
                                println!("Sim Time = {:?}",sim.sim_time);
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyP) => {
                        if let (Some(compute), Some(sim), Some(queue), Some(device)) =
                            (&self.compute, &self.sim, &self.queue, &self.device)
                        {
                            if key_state == ElementState::Released {
                                println!("Saved Particle States To CSV");
                                let particles = compute.gpu2cpu_particles(device,queue);
                                let motions = compute.gpu2cpu_motion(device,queue);

                                // Define the file path for writing data
                                let file_path_particle = r"C:\Users\fbfusco\particle_data.csv";
                                write_particles_to_csv(file_path_particle, &particles).unwrap();

                                let file_path_motion = r"C:\Users\fbfusco\motion_data.csv";
                                write_motion_to_csv(file_path_motion, &motions).unwrap();


                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(camera_controller) = &mut self.camera_controller {
                    camera_controller.process_scroll(delta);
                    true
                } else {
                    false
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            _ => false,
        }
    }

    fn update(&mut self, dt: instant::Duration) {
        if let (
            Some(camera),
            Some(camera_buffer),
            Some(camera_controller),
            Some(camera_uniform),
            Some(projection),
            Some(queue),
            Some(device),
            Some(compute),
            Some(sim),
        ) = (
            &mut self.camera,
            &self.camera_buffer,
            &mut self.camera_controller,
            &mut self.camera_uniform,
            &self.projection,
            &self.queue,
            &self.device,
            &self.compute,
            &mut self.sim, //made mutable so I can update sim_time and sim_idx
        ) {
            // Camera Controller Update
            camera_controller.update_camera(camera, dt);
            camera_uniform.update_view_proj(camera, projection);
            queue.write_buffer(
                camera_buffer,
                0,
                bytemuck::cast_slice(&[self.camera_uniform.expect("Camera uniform not init")]),
            );

            // Hydrodynamics Update

            // disturbance up[date logic]
            if sim.sim_time >= sim.disturbance[sim.sim_idx].simtime { // simtime is the disturbance field simtime
                compute.cpu2gpu_disturbance(queue, &sim.disturbance[sim.sim_idx]);
                sim.sim_idx += 1;
            }
            sim.sim_time += sim.params.dt;

            compute.compute_hash_grid(device, queue);
            //TODO: Sort spatial on GPU side
            // Get out the scattered spatial lookup
            let mut spatial = compute.gpu2cpu_spatial_scattered(device, queue);
            let mut start_indices = compute.gpu2cpu_start_indices(device, queue);
            let n = compute.num_particles as usize;
            let mut spatial_lookup = vec![(0, 0); n];
            for i in 0..n {
                spatial_lookup[i] = (spatial[i].index, spatial[i].key);
            }
            spatial_lookup.sort_by_cached_key(|k| k.1);
            let mut key_prev = spatial_lookup[0].1;
            start_indices[key_prev as usize] = 0;
            for i in 0..n {
                let key = spatial_lookup[i].1;
                if key != key_prev {
                    start_indices[key as usize] = i as u32;
                }
                key_prev = key;
                spatial[i].index = spatial_lookup[i].0;
                spatial[i].key = spatial_lookup[i].1;
            }
            // map sorted spatial and start indices back to gpu
            compute.cpu2gpu_spatial_sorted(queue, &spatial);
            compute.cpu2gpu_start_indices(queue, &start_indices);
            // continue with dynammics
            compute.compute_density_interpolant(device, queue);
            compute.compute_pressure_equation_of_state(device, queue);
            compute.compute_equation_of_motion(device, queue);
            compute.compute_leap_frog(device, queue);
            // Add in logic to update disturbance
            // sim.sim_time += sim.params.dt;
            // sim_idx += 1;
            // compute.cpu2gpu_disturbance(queue, &Your next disturbance field)

        }
        self.compute_particle_to_instance();
    }

    fn compute_particle_to_instance(&self) {
        if let (Some(device), Some(queue), Some(instance_pipeline)) =
            (&self.device, &self.queue, &self.particle_instance_pipeline)
        {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Comput Command Encoder"),
            });
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass Particle Instance"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(instance_pipeline);
            compute_pass.set_bind_group(
                0,
                self.particle_instance_bind_group
                    .as_ref()
                    .expect("Particle Instance not init"),
                &[],
            );
            compute_pass.dispatch_workgroups(
                (self
                    .sim
                    .as_ref()
                    .expect("sim not init")
                    .params
                    .num_particles
                    + 255)
                    / 256,
                1,
                1,
            );
            // Drop compute pass
            drop(compute_pass);
            // Submit commands to queue
            let command_buffer = encoder.finish();
            queue.submit([command_buffer]);
        }
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
            render_pass.set_bind_group(
                0,
                self.camera_bind_group
                    .as_ref()
                    .expect("Camera bind group not init"),
                &[],
            );
            render_pass.set_vertex_buffer(
                0,
                self.vertex_buffer
                    .as_ref()
                    .expect("Vertex buffer not init")
                    .slice(..),
            );
            render_pass.set_vertex_buffer(
                1,
                self.instance_buffer
                    .as_ref()
                    .expect("Instance buffer not init")
                    .slice(..),
            );
            render_pass.set_index_buffer(
                self.index_buffer
                    .as_ref()
                    .expect("Index buffer not init")
                    .slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(
                0..self.num_indices.expect("No vertex indices"),
                0,
                0..self
                    .sim
                    .as_ref()
                    .expect("sim not init")
                    .params
                    .num_particles,
            );
        }

        queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
