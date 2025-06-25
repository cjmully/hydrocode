use crate::shader_module::ShaderModuleBuilder;
use futures::executor::block_on;
use iced::widget::Shader;
use std::{num::NonZeroU64, str::FromStr};
use wgpu::{ShaderModule, util::DeviceExt};

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Particle {
    pub coord: [i32; 3],
    pub mass: f32,
    pub position: [f32; 3],
    pub density: f32,
    pub pressure: f32,
    pub smoothing_length: f32,
    pub material_idx: u32,
    pub _padding: f32,
    // 48 bytes
}

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct ParticleMotion {
    pub velocity: [f32; 3],
    pub drho_dt: f32,
    pub acceleration: [f32; 3],
    pub _padding: f32,
    pub velocity_p: [f32; 3],
    pub _padding2: f32,
    // 48 bytes
}

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Material {
    // Pressure Liquid EOS Parameters
    pub density_reference: f32,
    pub density_ref_threshold: f32,
    pub compressibility: f32,
    pub boundary_damping: f32,
    // Viscosity Parameters,
    pub cs: f32,
    pub alpha: f32,
    pub beta: f32,
    pub eps: f32,

    pub color: [f32; 4],
    // 48 bytes
}

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct SpatialLookup {
    pub index: u32,
    pub key: u32,
}

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct SimParams {
    pub grid_prime: [u32; 3],
    pub dt: f32,
    pub grid_size: f32,
    pub num_particles: u32,
    pub _padding: [f32; 2],
}

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Disturbance {
    pub local_position: [f32; 3],
    pub _padding: f32,
    pub local_velocity: [f32; 3],
    pub _padding2: f32,
    pub body_rates: [f32; 3],
    pub _padding3: f32,
    pub angular_accel: [f32; 3],
    pub _padding4: f32,
    pub linear_accel: [f32; 3],
    pub _padding5: f32,
    pub simtime: f32,
    pub _padding6: [f32; 7],
}

pub struct Sph {
    pub params: SimParams,
    pub disturbance: Disturbance,
    pub particles: Vec<Particle>,
    pub motion: Vec<ParticleMotion>,
    pub materials: Vec<Material>,
    // pub compute: Compute,
}

pub struct SphCompute {
    pub num_particles: u32,

    // Input Buffers
    pub buffer_particles: wgpu::Buffer,
    pub buffer_motion: wgpu::Buffer,
    pub buffer_materials: wgpu::Buffer,
    buffer_spatial_scattered: wgpu::Buffer,
    buffer_spatial_sorted: wgpu::Buffer,
    buffer_start_indices: wgpu::Buffer,
    pub buffer_params: wgpu::Buffer,

    // Uniform Buffers
    buffer_disturbance: wgpu::Buffer,

    // Staging Buffers
    staging_buffer_spatial: wgpu::Buffer,
    staging_buffer_start_indices: wgpu::Buffer,

    // Bind Groups
    bind_group_hash_grid: wgpu::BindGroup,
    bind_group_hydrodynamics: wgpu::BindGroup,
    bind_group_solver: wgpu::BindGroup,

    // Compute Pipeline
    compute_pipeline_hash_grid: wgpu::ComputePipeline,
    compute_pipeline_density_interpolant: wgpu::ComputePipeline,
    compute_pipeline_pressure_equation_of_state: wgpu::ComputePipeline,
    compute_pipeline_equation_of_motion: wgpu::ComputePipeline,
    compute_pipeline_leap_frog: wgpu::ComputePipeline,
}

impl Sph {
    pub fn new(
        params: SimParams,
        disturbance: Disturbance,
        particles: Vec<Particle>,
        motion: Vec<ParticleMotion>,
        materials: Vec<Material>,
    ) -> Self {
        Sph {
            params,
            disturbance,
            particles,
            motion,
            materials,
        }
    }
}

impl SphCompute {
    pub async fn new(device: &wgpu::Device, params: &SimParams) -> Self {
        let num_particles = params.num_particles as usize;
        const MATERIAL_MAX_LEN: usize = 4; // Hard coded, consider defining at compilation or user input

        // Create shader modules
        let description = include_str!("./description.wgsl");
        let util = include_str!("./util.wgsl");
        let kernel = include_str!("./kernel.wgsl");
        let hash_grid = include_str!("./hash_grid.wgsl");
        let hydrodynamics = include_str!("./hydrodynamics.wgsl");
        let solver = include_str!("./solver.wgsl");
        let module_hash_grid = ShaderModuleBuilder::new()
            .add_module(description)
            .add_module(hash_grid)
            .build(&device, Some("Shader Module Hash Grid"));
        let module_hydrodynamics = ShaderModuleBuilder::new()
            .add_module(util)
            .add_module(description)
            .add_module(kernel)
            .add_module(hydrodynamics)
            .build(&device, Some("Shader Module Hydrodynamics"));
        let module_solver = ShaderModuleBuilder::new()
            .add_module(description)
            .add_module(solver)
            .build(&device, Some("Shader Module Solver"));

        // Create Input Buffers
        let buffer_particles = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Buffer Particle"),
            size: (num_particles * std::mem::size_of::<Particle>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let buffer_motion = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Buffer Motion"),
            size: (num_particles * std::mem::size_of::<ParticleMotion>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let buffer_materials = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Buffer Material"),
            size: (MATERIAL_MAX_LEN * std::mem::size_of::<Material>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let buffer_spatial_scattered = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Buffer Spatial Lookup Scattered"),
            size: (num_particles * std::mem::size_of::<SpatialLookup>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let buffer_spatial_sorted = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Buffer Spatial Lookup Sorted"),
            size: (num_particles * std::mem::size_of::<SpatialLookup>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let buffer_start_indices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Buffer Start Indices"),
            size: (num_particles * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let buffer_params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Buffer Simulation Parameters"),
            size: std::mem::size_of::<SimParams>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Uniform Buffers
        let buffer_disturbance = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Buffer Disturbance"),
            size: std::mem::size_of::<Disturbance>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create Staging Buffers
        let staging_buffer_spatial = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer Spatial"),
            size: (num_particles * std::mem::size_of::<SpatialLookup>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let staging_buffer_start_indices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer Start Indices"),
            size: (num_particles * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Bind Group Layouts
        let bind_group_layout_hash_grid =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Bind Group Layout Hash grid"),
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
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let bind_group_layout_hydrodynamics =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Bind Group Layout Hydrodynamics"),
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
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let bind_group_layout_solver =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Bind Group Layout Solver"),
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

        // Bind Groups
        let bind_group_hash_grid = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind Group Hash Grid"),
            layout: &bind_group_layout_hash_grid,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_particles.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer_spatial_scattered.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffer_start_indices.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffer_params.as_entire_binding(),
                },
            ],
        });
        let bind_group_hydrodynamics = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind Group Hydrodynamics"),
            layout: &bind_group_layout_hydrodynamics,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_particles.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer_motion.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffer_materials.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffer_spatial_sorted.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: buffer_start_indices.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: buffer_params.as_entire_binding(),
                },
            ],
        });
        let bind_group_solver = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind Group Solver"),
            layout: &bind_group_layout_solver,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_particles.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer_motion.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffer_params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffer_disturbance.as_entire_binding(),
                },
            ],
        });

        // Pipeline Layouts
        let pipeline_layout_hash_grid =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pipeline Layout Hash Grid"),
                bind_group_layouts: &[&bind_group_layout_hash_grid],
                push_constant_ranges: &[],
            });
        let pipeline_layout_hydrodynamics =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pipeline Layout Hydrodynamics"),
                bind_group_layouts: &[&bind_group_layout_hydrodynamics],
                push_constant_ranges: &[],
            });
        let pipeline_layout_solver =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pipeline Layout Solver"),
                bind_group_layouts: &[&bind_group_layout_solver],
                push_constant_ranges: &[],
            });

        // Compute Pipeline
        let compute_pipeline_hash_grid =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline Hash Grid"),
                layout: Some(&pipeline_layout_hash_grid),
                module: &module_hash_grid,
                entry_point: Some("spatial_lookup"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        let compute_pipeline_density_interpolant =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline Density Interpolant"),
                layout: Some(&pipeline_layout_hydrodynamics),
                module: &module_hydrodynamics,
                entry_point: Some("density_interpolant"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        let compute_pipeline_pressure_equation_of_state =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline Pressure EOS"),
                layout: Some(&pipeline_layout_hydrodynamics),
                module: &module_hydrodynamics,
                entry_point: Some("pressure_equation_of_state"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        let compute_pipeline_equation_of_motion =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline Equation of Motion"),
                layout: Some(&pipeline_layout_hydrodynamics),
                module: &module_hydrodynamics,
                entry_point: Some("equation_of_motion"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        let compute_pipeline_leap_frog =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline Leap Frog"),
                layout: Some(&pipeline_layout_solver),
                module: &module_solver,
                entry_point: Some("leap_frog"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        SphCompute {
            num_particles: num_particles as u32,

            // Input Buffers
            buffer_particles,
            buffer_motion,
            buffer_materials,
            buffer_spatial_scattered,
            buffer_spatial_sorted,
            buffer_start_indices,

            // Uniform Buffers
            buffer_params,
            buffer_disturbance,

            // Staging Buffers
            staging_buffer_spatial,
            staging_buffer_start_indices,

            // Bind Groups
            bind_group_hash_grid,
            bind_group_hydrodynamics,
            bind_group_solver,

            // Compute Pipeline
            compute_pipeline_hash_grid,
            compute_pipeline_density_interpolant,
            compute_pipeline_pressure_equation_of_state,
            compute_pipeline_equation_of_motion,
            compute_pipeline_leap_frog,
        }
    }
}

impl SphCompute {
    pub fn cpu2gpu_particles(
        &self,
        queue: &wgpu::Queue,
        particles: &Vec<Particle>,
        motion: &Vec<ParticleMotion>,
    ) {
        queue.write_buffer(&self.buffer_particles, 0, bytemuck::cast_slice(&particles));
        queue.write_buffer(&self.buffer_motion, 0, bytemuck::cast_slice(&motion));
    }
    pub fn cpu2gpu_params(&self, queue: &wgpu::Queue, params: &SimParams) {
        queue.write_buffer(&self.buffer_params, 0, bytemuck::bytes_of(params));
    }
    pub fn cpu2gpu_materials(&self, queue: &wgpu::Queue, materials: &Vec<Material>) {
        queue.write_buffer(&self.buffer_materials, 0, bytemuck::cast_slice(&materials));
    }
    pub fn cpu2gpu_disturbance(&self, queue: &wgpu::Queue, disturbance: &Disturbance) {
        queue.write_buffer(&self.buffer_disturbance, 0, bytemuck::bytes_of(disturbance));
    }
    pub fn cpu2gpu_spatial_sorted(&self, queue: &wgpu::Queue, spatial: &Vec<SpatialLookup>) {
        queue.write_buffer(
            &self.buffer_spatial_sorted,
            0,
            bytemuck::cast_slice(&spatial),
        );
    }
    pub fn cpu2gpu_start_indices(&self, queue: &wgpu::Queue, start_indices: &Vec<u32>) {
        queue.write_buffer(
            &self.buffer_start_indices,
            0,
            bytemuck::cast_slice(&start_indices),
        );
    }

    pub fn gpu2cpu_spatial_scattered(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Vec<SpatialLookup> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder GPU to CPU Spatial Scattered"),
        });
        encoder.copy_buffer_to_buffer(
            &self.buffer_spatial_scattered,
            0,
            &self.staging_buffer_spatial,
            0,
            self.buffer_spatial_scattered.size(),
        );
        queue.submit(std::iter::once(encoder.finish()));
        // Read back buffer
        let buffer_slice = self.staging_buffer_spatial.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        // Wait for GPU to finish operation
        _ = device.poll(wgpu::PollType::Wait);
        // Read data from buffer
        let output_data = buffer_slice.get_mapped_range();
        // Convert to structure
        let spatial_out: Vec<SpatialLookup> = bytemuck::cast_slice(&output_data).to_vec();
        // Drop output and unmap staging buffer
        drop(output_data);
        self.staging_buffer_spatial.unmap();
        return spatial_out;
    }
    pub fn gpu2cpu_start_indices(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u32> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder GPU to CPU Start Indices"),
        });
        encoder.copy_buffer_to_buffer(
            &self.buffer_start_indices,
            0,
            &self.staging_buffer_start_indices,
            0,
            self.buffer_start_indices.size(),
        );
        queue.submit(std::iter::once(encoder.finish()));
        // Read back buffer
        let buffer_slice = self.staging_buffer_start_indices.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        // Wait for GPU to finish operation
        _ = device.poll(wgpu::PollType::Wait);
        // Read data from buffer
        let output_data = buffer_slice.get_mapped_range();
        // Convert to structure
        let start_indices_out: Vec<u32> = bytemuck::cast_slice(&output_data).to_vec();
        // Drop output and unmap staging buffer
        drop(output_data);
        self.staging_buffer_start_indices.unmap();
        return start_indices_out;
    }

    pub fn compute_hash_grid(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder Hash Grid"),
        });
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass Hash Grid"),
            timestamp_writes: None,
        });
        // Setup compute pass commands
        compute_pass.set_pipeline(&self.compute_pipeline_hash_grid);
        compute_pass.set_bind_group(0, &self.bind_group_hash_grid, &[]);
        compute_pass.dispatch_workgroups((self.num_particles + 255) / 256, 1, 1);
        // Drop compute pass to gain access to encoder again
        drop(compute_pass);
        // Submit commands to queue
        let command_buffer = encoder.finish();
        queue.submit([command_buffer]);
    }
    pub fn compute_density_interpolant(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder Density Interpolant"),
        });
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass Density Interpolant"),
            timestamp_writes: None,
        });
        // Setup compute pass commands
        compute_pass.set_pipeline(&self.compute_pipeline_density_interpolant);
        compute_pass.set_bind_group(0, &self.bind_group_hydrodynamics, &[]);
        compute_pass.dispatch_workgroups((self.num_particles + 255) / 256, 1, 1);
        // Drop compute pass to gain access to encoder again
        drop(compute_pass);
        // Submit commands to queue
        let command_buffer = encoder.finish();
        queue.submit([command_buffer]);
    }
    pub fn compute_pressure_equation_of_state(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder Pressure EOS"),
        });
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass Pressure EOS"),
            timestamp_writes: None,
        });
        // Setup compute pass commands
        compute_pass.set_pipeline(&self.compute_pipeline_pressure_equation_of_state);
        compute_pass.set_bind_group(0, &self.bind_group_hydrodynamics, &[]);
        compute_pass.dispatch_workgroups((self.num_particles + 255) / 256, 1, 1);
        // Drop compute pass to gain access to encoder again
        drop(compute_pass);
        // Submit commands to queue
        let command_buffer = encoder.finish();
        queue.submit([command_buffer]);
    }
    pub fn compute_equation_of_motion(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder Equation of Motion"),
        });
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass Equation of Motion"),
            timestamp_writes: None,
        });
        // Setup compute pass commands
        compute_pass.set_pipeline(&self.compute_pipeline_equation_of_motion);
        compute_pass.set_bind_group(0, &self.bind_group_hydrodynamics, &[]);
        compute_pass.dispatch_workgroups((self.num_particles + 255) / 256, 1, 1);
        // Drop compute pass to gain access to encoder again
        drop(compute_pass);
        // Submit commands to queue
        let command_buffer = encoder.finish();
        queue.submit([command_buffer]);
    }
    pub fn compute_leap_frog(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder Leap Frog"),
        });
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass Leap Frog"),
            timestamp_writes: None,
        });
        // Setup compute pass commands
        compute_pass.set_pipeline(&self.compute_pipeline_leap_frog);
        compute_pass.set_bind_group(0, &self.bind_group_solver, &[]);
        compute_pass.dispatch_workgroups((self.num_particles + 255) / 256, 1, 1);
        // Drop compute pass to gain access to encoder again
        drop(compute_pass);
        // Submit commands to queue
        let command_buffer = encoder.finish();
        queue.submit([command_buffer]);
    }
}
