use wgpu::*;

pub struct MlsMpm {
    device: wgpu::Device,
    queue: wgpu::Queue,

    // Input Buffers
    buffer_particles: wgpu::Buffer,
}
