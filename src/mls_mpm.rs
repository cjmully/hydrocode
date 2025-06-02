use crate::shader_module::ShaderModuleBuilder;
use futures::executor::block_on;
use std::{num::NonZeroU64, str::FromStr};
use wgpu::{ShaderModule, util::DeviceExt};

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Particle {
    pub position: [f32; 3],
    pub mass: f32,
    pub velocity: [f32; 3],
    pub material_idx: u32,
    pub C: [f32; 12],
}

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Grid {
    pub vx: i32,
    pub vy: i32,
    pub vz: i32,
    pub mass: i32,
}

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct SimParams {
    pub grid_resolution: u32,
    pub dt: f32,
    pub scale_distance: f32,
    pub num_particles: u32,
    pub num_nodes: u32,
    pub _padding: u32,
}

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Material {
    pub eos_density: f32,       // reference density
    pub eos_threshold: f32,     // negative pressure threshold
    pub eos_stiffness: f32,     // stiffness coefficient
    pub eos_n: f32,             // exponent
    pub dynamic_viscosity: f32, // viscosity coefficient
    pub _padding: u32,
}

pub struct MlsMpm {
    pub params: SimParams,
    pub particles: Vec<Particle>,
    pub materials: Vec<Material>,
    // pub compute: Compute,
}

pub struct MlsMpmCompute {
    num_particles: u32,
    // Input Buffers
    pub buffer_particles: wgpu::Buffer,
    buffer_grid: wgpu::Buffer,
    buffer_materials: wgpu::Buffer,

    // Uniform Buffers
    buffer_params: wgpu::Buffer,

    // Staging Buffers
    staging_buffer_particles: wgpu::Buffer,
    staging_buffer_grid: wgpu::Buffer,

    // Bind Groups
    bind_group_particle_to_grid: wgpu::BindGroup,
    bind_group_particle_constitutive_model: wgpu::BindGroup,
    bind_group_grid_to_particle: wgpu::BindGroup,
    bind_group_grid_update: wgpu::BindGroup,

    // Compute Pipeline
    compute_pipeline_particle_to_grid: wgpu::ComputePipeline,
    compute_pipeline_particle_constitutive_model: wgpu::ComputePipeline,
    compute_pipeline_grid_to_particle: wgpu::ComputePipeline,
    compute_pipeline_grid_update: wgpu::ComputePipeline,
}

impl MlsMpm {
    pub fn new(params: SimParams, particles: Vec<Particle>, materials: Vec<Material>) -> Self {
        MlsMpm {
            params,
            particles,
            materials,
        }
    }
}

impl MlsMpmCompute {
    pub async fn new(device: &wgpu::Device, params: &SimParams) -> Self {
        let num_particles = params.num_particles as usize;
        let num_nodes = params.num_nodes as usize;
        const MATERIAL_MAX_LEN: usize = 25; // Hard coded, consider defining at compilation or user input

        // Create shader modules
        let util = include_str!("./util.wgsl");
        let grid_reset = include_str!("./grid_reset.wgsl");
        let particle_to_grid = include_str!("./particle_to_grid.wgsl");
        let particle_constitutive_model = include_str!("./particle_constitutive_model.wgsl");
        let grid_to_particle = include_str!("./grid_to_particle.wgsl");
        let grid_update = include_str!("./grid_update.wgsl");
        let grid_reset_module = ShaderModuleBuilder::new()
            .add_module(grid_reset)
            .build(&device, Some("Shader Module Grid Reset"));
        let module_particle_to_grid = ShaderModuleBuilder::new()
            .add_module(particle_to_grid)
            .add_module(util)
            .build(&device, Some("Shader Module Particle to Grid"));
        let module_particle_constitutive_model = ShaderModuleBuilder::new()
            .add_module(particle_constitutive_model)
            .add_module(util)
            .build(&device, Some("Shader Module Particle Constitutive Model"));
        let module_grid_to_particle = ShaderModuleBuilder::new()
            .add_module(grid_to_particle)
            .add_module(util)
            .build(&device, Some("Shader Module Grid to Particle"));
        let module_grid_update = ShaderModuleBuilder::new()
            .add_module(grid_update)
            .add_module(util)
            .build(&device, Some("Shader Module Grid Update"));

        // Create Input Buffers
        let buffer_particles = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Buffer Particle"),
            size: (num_particles * std::mem::size_of::<Particle>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let buffer_grid = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer Grid"),
            contents: &vec![0u8; num_nodes * std::mem::size_of::<Grid>()],
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        let buffer_materials = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Buffer Material"),
            size: (MATERIAL_MAX_LEN * std::mem::size_of::<Material>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Uniform Buffers
        let buffer_params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Buffer Simulation Parameters"),
            size: std::mem::size_of::<SimParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create Staging Buffers
        let staging_buffer_particles = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer Particle"),
            size: (num_particles * std::mem::size_of::<Particle>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let staging_buffer_grid = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer Grid"),
            size: (num_nodes * std::mem::size_of::<Grid>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Bind Group Layouts
        let bind_group_layout_particle_to_grid =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Bind Group Layout Particle to Grid"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            min_binding_size: None,
                            has_dynamic_offset: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let bind_group_layout_particle_constitutive_model =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Bind Group Layout Particle Constitutive Model"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            min_binding_size: None,
                            has_dynamic_offset: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
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
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let bind_group_layout_grid_to_particle =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Bind Group Layout Grid to Particle"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            min_binding_size: None,
                            has_dynamic_offset: false,
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
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let bind_group_layout_grid_update =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Bind Group Layout Grid Update"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Bind Groups
        let bind_group_particle_to_grid = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind Group Particle to Grid"),
            layout: &bind_group_layout_particle_to_grid,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_particles.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer_grid.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffer_params.as_entire_binding(),
                },
            ],
        });

        let bind_group_particle_constitutive_model =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Bind Group Particle Constitutive Model"),
                layout: &bind_group_layout_particle_constitutive_model,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer_particles.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: buffer_grid.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: buffer_materials.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: buffer_params.as_entire_binding(),
                    },
                ],
            });

        let bind_group_grid_to_particle = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind Group Grid to Particle"),
            layout: &bind_group_layout_grid_to_particle,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_particles.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer_grid.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffer_params.as_entire_binding(),
                },
            ],
        });

        let bind_group_grid_update = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind Group Grid Update"),
            layout: &bind_group_layout_grid_update,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_grid.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer_params.as_entire_binding(),
                },
            ],
        });

        // Pipeline Layouts
        let pipeline_layout_particle_to_grid =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pipeline Layout Particle to Grid"),
                bind_group_layouts: &[&bind_group_layout_particle_to_grid],
                push_constant_ranges: &[],
            });

        let pipeline_layout_particle_constitutive_model =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pipeline Layout Particle Constitutive Model"),
                bind_group_layouts: &[&bind_group_layout_particle_constitutive_model],
                push_constant_ranges: &[],
            });

        let pipeline_layout_grid_to_particle =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pipeline Layout Grid to Particle"),
                bind_group_layouts: &[&bind_group_layout_grid_to_particle],
                push_constant_ranges: &[],
            });

        let pipeline_layout_grid_update =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pipeline Layout Grid Update"),
                bind_group_layouts: &[&bind_group_layout_grid_update],
                push_constant_ranges: &[],
            });

        // Compute Pipeline
        let compute_pipeline_particle_to_grid =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline Particle to Grid"),
                layout: Some(&pipeline_layout_particle_to_grid),
                module: &module_particle_to_grid,
                entry_point: Some("particle_to_grid"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let compute_pipeline_particle_constitutive_model =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline Particle Constitutive Model"),
                layout: Some(&pipeline_layout_particle_constitutive_model),
                module: &module_particle_constitutive_model,
                entry_point: Some("particle_constitutive_model"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let compute_pipeline_grid_to_particle =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline Grid to Particle"),
                layout: Some(&pipeline_layout_grid_to_particle),
                module: &module_grid_to_particle,
                entry_point: Some("grid_to_particle"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let compute_pipeline_grid_update =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline Grid Update"),
                layout: Some(&pipeline_layout_grid_update),
                module: &module_grid_update,
                entry_point: Some("grid_update"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        MlsMpmCompute {
            num_particles: num_particles as u32,
            // Input Buffers
            buffer_particles,
            buffer_grid,
            buffer_materials,

            // Uniform Buffers
            buffer_params,

            // Staging Buffers
            staging_buffer_particles,
            staging_buffer_grid,

            // Bind Groups
            bind_group_particle_to_grid,
            bind_group_grid_to_particle,
            bind_group_particle_constitutive_model,
            bind_group_grid_update,

            // Compute Pipeline
            compute_pipeline_particle_to_grid,
            compute_pipeline_particle_constitutive_model,
            compute_pipeline_grid_to_particle,
            compute_pipeline_grid_update,
        }
    }
}

impl MlsMpmCompute {
    pub fn cpu2gpu_particles(&self, queue: &wgpu::Queue, particles: &Vec<Particle>) {
        queue.write_buffer(&self.buffer_particles, 0, bytemuck::cast_slice(&particles));
    }
    pub fn cpu2gpu_params(&self, queue: &wgpu::Queue, params: &SimParams) {
        queue.write_buffer(&self.buffer_params, 0, bytemuck::bytes_of(params));
    }
    pub fn cpu2gpu_materials(&self, queue: &wgpu::Queue, materials: &Vec<Material>) {
        queue.write_buffer(&self.buffer_materials, 0, bytemuck::cast_slice(&materials));
    }

    pub fn gpu2cpu_particles(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<Particle> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder GPU to CPU Particles"),
        });
        encoder.copy_buffer_to_buffer(
            &self.buffer_particles,
            0,
            &self.staging_buffer_particles,
            0,
            self.buffer_particles.size(),
        );
        queue.submit(std::iter::once(encoder.finish()));
        // Read back buffer
        let buffer_slice = self.staging_buffer_particles.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        // Wait for GPU to finish operation
        _ = device.poll(wgpu::PollType::Wait);
        // Read data from buffer
        let output_data = buffer_slice.get_mapped_range();
        // Convert to structure
        let particles_out: Vec<Particle> = bytemuck::cast_slice(&output_data).to_vec();
        // Drop output and unmap staging buffer
        drop(output_data);
        self.staging_buffer_particles.unmap();
        return particles_out;
    }

    pub fn gpu2cpu_grid(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<Grid> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder GPU to CPU Grid"),
        });
        encoder.copy_buffer_to_buffer(
            &self.buffer_grid,
            0,
            &self.staging_buffer_grid,
            0,
            self.buffer_grid.size(),
        );
        queue.submit(std::iter::once(encoder.finish()));
        // Read back buffer
        let buffer_slice = self.staging_buffer_grid.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        // Wait for GPU to finish operation
        _ = device.poll(wgpu::PollType::Wait);
        // Read data from buffer
        let output_data = buffer_slice.get_mapped_range();
        // Convert to structure
        let grid_out: Vec<Grid> = bytemuck::cast_slice(&output_data).to_vec();
        // Drop output and unmap staging buffer
        drop(output_data);
        self.staging_buffer_grid.unmap();
        return grid_out;
    }

    pub fn compute_particle_to_grid(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder Particle to Grid"),
        });
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass Particle to Grid"),
            timestamp_writes: None,
        });
        // Setup compute pass commands
        compute_pass.set_pipeline(&self.compute_pipeline_particle_to_grid);
        compute_pass.set_bind_group(0, &self.bind_group_particle_to_grid, &[]);
        compute_pass.dispatch_workgroups((self.num_particles + 255) / 256, 1, 1);
        // Drop compute pass to gain access to encoder again
        drop(compute_pass);
        // Submit commands to queue
        let command_buffer = encoder.finish();
        queue.submit([command_buffer]);
    }

    pub fn compute_particle_constitutive_model(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder Particle Constitutive Model"),
        });
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass Particle Constitutive Model"),
            timestamp_writes: None,
        });
        // Setup compute pass commands
        compute_pass.set_pipeline(&self.compute_pipeline_particle_constitutive_model);
        compute_pass.set_bind_group(0, &self.bind_group_particle_constitutive_model, &[]);
        compute_pass.dispatch_workgroups((self.num_particles + 255) / 256, 1, 1);
        // Drop compute pass to gain access to encoder again
        drop(compute_pass);
        // Submit commands to queue
        let command_buffer = encoder.finish();
        queue.submit([command_buffer]);
    }

    pub fn compute_grid_to_particle(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder Grid to Particle"),
        });
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass Grid to Particle"),
            timestamp_writes: None,
        });
        // Setup compute pass commands
        compute_pass.set_pipeline(&self.compute_pipeline_grid_to_particle);
        compute_pass.set_bind_group(0, &self.bind_group_grid_to_particle, &[]);
        compute_pass.dispatch_workgroups((self.num_particles + 255) / 256, 1, 1);
        // Drop compute pass to gain access to encoder again
        drop(compute_pass);
        // Submit commands to queue
        let command_buffer = encoder.finish();
        queue.submit([command_buffer]);
    }

    pub fn compute_grid_update(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder Grid Update"),
        });
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass Grid Update"),
            timestamp_writes: None,
        });
        // Setup compute pass commands
        compute_pass.set_pipeline(&self.compute_pipeline_grid_update);
        compute_pass.set_bind_group(0, &self.bind_group_grid_update, &[]);
        compute_pass.dispatch_workgroups((self.num_particles + 255) / 256, 1, 1);
        // Drop compute pass to gain access to encoder again
        drop(compute_pass);
        // Submit commands to queue
        let command_buffer = encoder.finish();
        queue.submit([command_buffer]);
    }
}
