struct Grid {
    vx: atomic<i32>,
    vy: atomic<i32>,
    vz: atomic<i32>,
    mass: atomic<i32>,
}

struct Particle {
    position: vec3f,
    mass: f32,
    velocity: vec3f,
    _padding: u32,
    C: mat3x3f, // MLS-MPM Affine Matrix
}

struct SimParams {
    grid_resolution: u32,
    dt: f32,
    scale_distance:f32,
    num_particles: u32,
    num_nodes: u32,
    _padding: u32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> grid: array<Grid>;
@group(0) @binding(2) var<uniform> params: SimParams;

@compute @workgroup_size(256)
fn particle_to_grid(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.num_particles) {
        return;
    }
    let particle = particles[idx];
    // Get Quadratic Weights
    let grid_res = f32(params.grid_resolution);
    let node_coord: vec3f = floor(particle.position * grid_res + 1e-7);
    let node_dist: vec3f = particle.position * grid_res - node_coord - 0.5;
    let weights = quadratic_weights(node_dist);

    for (var gx = 0u; gx < 3; gx++) {
        for (var gy = 0u; gy < 3; gy++) {
            for (var gz = 0u; gz < 3; gz++) {
                let weight = weights[gx].x * weights[gy].y * weights[gz].z;
                let neighbor_coord = vec3f(
                    node_coord.x + f32(gx) - 1.0,
                    node_coord.y + f32(gy) - 1.0,
                    node_coord.z + f32(gz) - 1.0);
                let neighbor_dist = neighbor_coord - particle.position * grid_res + 0.5;
                let Q: vec3f = particle.C * neighbor_dist;
                // Compute influence on grid from particle
                let mass_influence = weight * particle.mass;
                let velocity_influence = mass_influence * (particle.velocity + Q);
                let node_idx = get_node_index(neighbor_coord, params.grid_resolution);
                // Update Grid State
                atomicAdd(&grid[node_idx].mass, f32_to_i32(mass_influence));
                atomicAdd(&grid[node_idx].vx, f32_to_i32(velocity_influence.x));
                atomicAdd(&grid[node_idx].vy, f32_to_i32(velocity_influence.y));
                atomicAdd(&grid[node_idx].vz, f32_to_i32(velocity_influence.z));

                // atomicAdd(&grid[node_idx].mass, 1i);
                // atomicAdd(&grid[node_idx].vx, 1i);
                // atomicAdd(&grid[node_idx].vy, 1i);
                // atomicAdd(&grid[node_idx].vz, 1i);
            }
        }
    }
}
