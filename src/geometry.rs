use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct SphereVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl SphereVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SphereVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Normal
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Texture coordinates
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub struct SphereGeometry {
    pub vertices: Vec<SphereVertex>,
    pub indices: Vec<u32>,
}

impl SphereGeometry {
    /// Create a new sphere with the given radius and tessellation levels
    ///
    /// # Arguments
    /// * `radius` - The radius of the sphere
    /// * `latitude_segments` - Number of horizontal segments (rings)
    /// * `longitude_segments` - Number of vertical segments (slices)
    pub fn new(radius: f32, latitude_segments: u32, longitude_segments: u32) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Add top pole vertex
        vertices.push(SphereVertex {
            position: [0.0, radius, 0.0],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [0.5, 0.0],
        });

        // Generate vertices for middle rings (excluding poles)
        for lat in 1..latitude_segments {
            let theta = lat as f32 * PI / latitude_segments as f32; // 0 to PI (top to bottom)
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            for lon in 0..longitude_segments {
                let phi = lon as f32 * 2.0 * PI / longitude_segments as f32; // 0 to 2*PI (around)
                let sin_phi = phi.sin();
                let cos_phi = phi.cos();

                // Spherical coordinates to Cartesian
                let x = sin_theta * cos_phi;
                let y = cos_theta;
                let z = sin_theta * sin_phi;

                let position = [x * radius, y * radius, z * radius];
                let normal = [x, y, z];

                // Texture coordinates
                let u = lon as f32 / longitude_segments as f32;
                let v = lat as f32 / latitude_segments as f32;
                let tex_coords = [u, v];

                vertices.push(SphereVertex {
                    position,
                    normal,
                    tex_coords,
                });
            }
        }

        // Add bottom pole vertex
        vertices.push(SphereVertex {
            position: [0.0, -radius, 0.0],
            normal: [0.0, -1.0, 0.0],
            tex_coords: [0.5, 1.0],
        });

        // Generate indices

        // Top cap triangles (connecting top pole to first ring)
        for lon in 0..longitude_segments {
            let next_lon = (lon + 1) % longitude_segments;

            indices.push(0); // Top pole
            indices.push(1 + lon); // Next vertex in first ring
            indices.push(1 + next_lon); // Current vertex in first ring
        }

        // Middle rings (quads split into triangles)
        for lat in 0..(latitude_segments - 2) {
            for lon in 0..longitude_segments {
                let next_lon = (lon + 1) % longitude_segments;

                // Current ring starts at index 1 + lat * longitude_segments
                let current_ring_start = 1 + lat * longitude_segments;
                let next_ring_start = 1 + (lat + 1) * longitude_segments;

                let current = current_ring_start + lon;
                let current_next = current_ring_start + next_lon;
                let next = next_ring_start + lon;
                let next_next = next_ring_start + next_lon;

                // First triangle
                indices.push(current);
                indices.push(current_next);
                indices.push(next);

                // Second triangle
                indices.push(current_next);
                indices.push(next_next);
                indices.push(next);
            }
        }

        // Bottom cap triangles (connecting last ring to bottom pole)
        let bottom_pole_index = vertices.len() as u32 - 1;
        let last_ring_start = 1 + (latitude_segments - 2) * longitude_segments;

        for lon in 0..longitude_segments {
            let next_lon = (lon + 1) % longitude_segments;

            indices.push(bottom_pole_index); // Bottom pole
            indices.push(last_ring_start + next_lon); // Next vertex in last ring
            indices.push(last_ring_start + lon); // Current vertex in last ring
        }

        Self { vertices, indices }
    }

    /// Create a simple sphere with default tessellation (good for most cases)
    pub fn default_sphere(radius: f32) -> Self {
        Self::new(radius, 32, 64)
    }

    /// Create a low-poly sphere (fewer triangles, better performance)
    pub fn low_poly_sphere(radius: f32) -> Self {
        Self::new(radius, 8, 16)
    }

    /// Create a high-poly sphere (more triangles, smoother surface)
    pub fn high_poly_sphere(radius: f32) -> Self {
        Self::new(radius, 64, 128)
    }

    /// Get the number of vertices
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Get the number of indices
    pub fn index_count(&self) -> usize {
        self.indices.len()
    }

    /// Get the number of triangles
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Create wgpu vertex buffer from the vertices
    pub fn create_vertex_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        use wgpu::util::DeviceExt;
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sphere Vertex Buffer"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        })
    }

    /// Create wgpu index buffer from the indices
    pub fn create_index_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        use wgpu::util::DeviceExt;
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sphere Index Buffer"),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX,
        })
    }
}

// Example usage and helper functions
impl SphereGeometry {
    /// Create a complete sphere setup for wgpu rendering
    pub fn create_render_data(&self, device: &wgpu::Device) -> SphereRenderData {
        let vertex_buffer = self.create_vertex_buffer(device);
        let index_buffer = self.create_index_buffer(device);
        let num_indices = self.indices.len() as u32;

        SphereRenderData {
            vertex_buffer,
            index_buffer,
            num_indices,
        }
    }
}

pub struct SphereRenderData {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sphere_creation() {
        let sphere = SphereGeometry::new(1.0, 4, 8);

        // Should have (4+1) * (8+1) = 45 vertices
        assert_eq!(sphere.vertex_count(), 45);

        // Should have 4 * 8 * 6 = 192 indices (2 triangles per quad, 3 indices per triangle)
        assert_eq!(sphere.index_count(), 192);

        // Should have 64 triangles
        assert_eq!(sphere.triangle_count(), 64);
    }

    #[test]
    fn test_sphere_radius() {
        let radius = 2.5;
        let sphere = SphereGeometry::new(radius, 8, 16);

        // Check that vertices are approximately at the correct distance from origin
        for vertex in &sphere.vertices {
            let [x, y, z] = vertex.position;
            let distance = (x * x + y * y + z * z).sqrt();
            assert!((distance - radius).abs() < 0.001);
        }
    }
}
