
@group(0) @binding(0)
var<storage, read_write> rigid_particles: array<RigidParticle>;

@group(0) @binding(1)
var<storage, read_write> rigid_bodies: array<RigidBody>;

@group(0) @binding(2)
var<storage, read> spatial: array<SpatialLookup>;

@group(0) @binding(3)
var<storage, read> start_indices: array<u32>;

@group(0) @binding(4)
var<storage, read> params: SimParams;

@compute @workgroup_size(256)
fn volume_interpolant(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.num_rigid_particles) {
        return;
    }
    let num_particles = params.num_particles;
    let num_total_particles = params.num_total_particles;
    // Get particle
    let particle = rigid_particles[index];
    // Get prime values
    let prime = params.grid_prime;
    // Get paticle parameters
    let h_a = particle.smoothing_length;
    // Initialize Volume
    var volume: f32 = 0.0;
    // Loop through all adjacent grid coordinates to particle
    for (var gx = -1i; gx < 2; gx++) {
        for (var gy = -1i; gy < 2; gy++) {
            for (var gz = -1i; gz < 2; gz++) {
            // let gz = 0i;
                // Calculate hash key
                let grid_coord_x = particle.coord.x + gx;
                let grid_coord_y = particle.coord.y + gy;
                let grid_coord_z = particle.coord.z + gz;
                let key_x = u32(grid_coord_x) * prime.x;
                let key_y = u32(grid_coord_y) * prime.y;
                let key_z = u32(grid_coord_z) * prime.z;
                let key = (key_x + key_y + key_z) % num_total_particles;
                // Find start index in particle list and loop through neihbors
                let idx0 = start_indices[key];
                var spatial_idx: u32 = u32(0);
                for (spatial_idx = idx0; spatial_idx < num_total_particles; spatial_idx++) {
                    // break if spatial key != particle key
                    // particles[index].material_idx += 1u;
                    if (spatial[spatial_idx].key != key) {
                        break;
                    }
                    let neighbor_idx = spatial[spatial_idx].index - num_particles;
                    let neighbor = rigid_particles[neighbor_idx];
                    // Check if neighbor belongs to the same rigid body
                    if (particle.body_idx == neighbor.body_idx) {
                        // Compute distance to neighbor
                        let rvec_ab = get_rigid_particle_distance(particle,neighbor,params.grid_size);
                        // let rvec_ab = (vec3f(particle.coord - neighbor.coord) + particle.position - neighbor.position) * params.grid_size;
                        let r2_ab = dot(rvec_ab,rvec_ab);
                        let r_ab = sqrt(r2_ab);
                        // Check if neighbor is within smoothing length
                        let h_ab = 0.5 * (h_a + neighbor.smoothing_length);
                        let h2_ab = h_ab * h_ab;
                        let kernel = kernel_cubic_bspline(r_ab, r2_ab, h_ab, h2_ab);
                        volume += 1.0 / kernel;
                    }
                }
            }
        }
    }
    // Update final volume interpolant
    rigid_particles[index].volume = volume;
}

