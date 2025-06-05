
struct Grid {
    vx: i32,
    vy: i32,
    vz: i32,
    mass: i32,
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
    scale_distance: f32,
    num_particles: u32,
    num_nodes: u32,
    _padding: u32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<storage, read> grid: array<Grid>;
@group(0) @binding(2) var<uniform> params: SimParams;

@compute @workgroup_size(256)
fn grid_to_particle(@builtin(global_invocation_id) global_id: vec3<u32>) {
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
    // Reinitialize Velocity & Affine Matrix
    var velocity = vec3f(0.0);
    var C: mat3x3f = mat3x3f(vec3f(0.0),vec3f(0.0),vec3f(0.0));
    for (var gx = 0u; gx < 3; gx++) {
        for (var gy = 0u; gy < 3; gy++) {
            for (var gz = 0u; gz < 3; gz++) {
                let weight = weights[gx].x * weights[gy].y * weights[gz].z;
                let neighbor_coord = vec3f(
                    node_coord.x + f32(gx) - 1.0,
                    node_coord.y + f32(gy) - 1.0,
                    node_coord.z + f32(gz) - 1.0);
                let neighbor_dist = neighbor_coord - particle.position * grid_res + 0.5;
                let node_idx = get_node_index(neighbor_coord, params.grid_resolution);
                let neighbor_node = grid[node_idx];
                // Compute Velocity to map back to particles
                velocity += vec3f(
                    i32_to_f32(neighbor_node.vx),
                    i32_to_f32(neighbor_node.vy),
                    i32_to_f32(neighbor_node.vz),    
                ) * weight;
                C += mat3x3f(
                    velocity * neighbor_dist.x,
                    velocity * neighbor_dist.y,
                    velocity * neighbor_dist.z,
                );
            }
        }
    }
    // Update particle velocity and affine matrix
    particles[idx].velocity = velocity;
    particles[idx].C = C * 4.0;

    // Advect particles
    particles[idx].position += particles[idx].velocity * params.dt;

    // Boundary Conditions (maby have this as separate dispatch)
    // Need to use buffer for parameters insated of hard code
    let k = 2.0;  
    let wallStiffness = 1.0;
    let x_n: vec3f = particles[idx].position + particles[idx].velocity * params.dt * k;
    let wallMin: vec3f = vec3f(3.0 / f32(params.grid_resolution));
    let wallMax: vec3f = vec3f(1.0 - 4.0 / f32(params.grid_resolution));
    // if (x_n.x < wallMin.x) { particles[idx].velocity.x += wallStiffness * (wallMin.x - x_n.x); }
    // if (x_n.x > wallMax.x) { particles[idx].velocity.x += wallStiffness * (wallMax.x - x_n.x); }
    // if (x_n.y < wallMin.y) { particles[idx].velocity.y += wallStiffness * (wallMin.y - x_n.y); }
    // if (x_n.y > wallMax.y) { particles[idx].velocity.y += wallStiffness * (wallMax.y - x_n.y); }
    // if (x_n.z < wallMin.z) { particles[idx].velocity.z += wallStiffness * (wallMin.z - x_n.z); }
    // if (x_n.z > wallMax.z) { particles[idx].velocity.z += wallStiffness * (wallMax.z - x_n.z); }
}
