
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
    material_idx: u32,
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

struct Material {
    eos_density: f32, // reference density
    eos_threshold: f32, // negative pressure threshold
    eos_stiffness: f32, // stiffness coefficient
    eos_n: f32, // exponent 
    dynamic_viscosity: f32, // viscosity coefficient
    _padding: u32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> grid: array<Grid>;
@group(0) @binding(2) var<storage, read> materials: array<Material>;
@group(0) @binding(3) var<uniform> params: SimParams;

@compute @workgroup_size(256)
fn particle_constitutive_model(@builtin(global_invocation_id) global_id: vec3<u32>) {
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
    // First pass through particle/neighbor weighting to compute density
    // Initialize density
    var density: f32 = 0.0;
    for (var gx = 0u; gx < 3; gx++) {
        for (var gy = 0u; gy < 3; gy++) {
            for (var gz = 0u; gz < 3; gz++) {
                let weight = weights[gx].x * weights[gy].y * weights[gz].z;
                let neighbor_coord = vec3f(
                    node_coord.x + f32(gx) - 1.0,
                    node_coord.y + f32(gy) - 1.0,
                    node_coord.z + f32(gz) - 1.0);
                let node_idx = get_node_index(neighbor_coord, params.grid_resolution);
                density += i32_to_f32(grid[node_idx].mass) * weight;
            }
        }
    }
    // Compute constitutive model
    let material = materials[particle.material_idx];
    let volume = particle.mass / density;
    // Pressure Equation of State
    let pressure = max(-material.eos_threshold,
         material.eos_stiffness * (pow(density - material.eos_density, material.eos_n) - 1.0));
    var stress: mat3x3f = mat3x3f(vec3f(-pressure,0.0,0.0),
                                vec3f(0.0,-pressure,0.0),
                                vec3f(0.0,0.0,-pressure));
    let dudv = particle.C;
    let strain = dudv + transpose(dudv);
    let viscosity = material.dynamic_viscosity * strain;
    stress += viscosity;
    let eq_16_0 = -volume * 4.0 * stress * params.dt;
    // Second pass through particle/neighbor weighting to update grid momentum
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
                let momentum: vec3f = eq_16_0 * weight * neighbor_dist;
                // Update momentum using atomics
                atomicAdd(&grid[node_idx].vx, f32_to_i32(momentum.x));
                atomicAdd(&grid[node_idx].vy, f32_to_i32(momentum.y));
                atomicAdd(&grid[node_idx].vz, f32_to_i32(momentum.z));
            }
        }
    }
}
