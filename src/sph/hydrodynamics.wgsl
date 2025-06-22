
@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<storage, read_write> particles_motion: array<ParticleMotion>;

@group(0) @binding(2)
var<storage, read> material: array<Material>;

@group(0) @binding(3)
var<storage, read> spatial: array<SpatialLookup>;

@group(0) @binding(4)
var<storage, read> start_indices: array<u32>;

@group(0) @binding(5)
var<storage, read_write> params: SimParams;

@compute @workgroup_size(256)
fn density_interpolant(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    let num_particles = params.num_particles;
    if (index >= num_particles) {
        return;
    }
    // Get particle
    let particle = particles[index];
    // Get prime values
    let prime = params.grid_prime;
    // Get paticle parameters
    let h_a = particle.smoothing_length;
    let mass_a = particle.mass;
    // Initialize Density
    var density: f32 = mass_a / (PI * h_a * h_a);
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
                let key = (key_x + key_y + key_z) % num_particles;
                // Find start index in particle list and loop through neihbors
                let idx0 = start_indices[key];
                var spatial_idx: u32 = u32(0);
                for (spatial_idx = idx0; spatial_idx < num_particles; spatial_idx++) {
                    // break if spatial key != particle key
                    // particles[index].material_idx += 1u;
                    if (spatial[spatial_idx].key != key) {
                        break;
                    }
                    let neighbor_idx = spatial[spatial_idx].index;
                    let neighbor = particles[neighbor_idx];
                    // Compute distance to neighbor
                    let rvec_ab = get_particle_distance(particle,neighbor,params.grid_size);
                    // let rvec_ab = (vec3f(particle.coord - neighbor.coord) + particle.position - neighbor.position) * params.grid_size;
                    let r2_ab = dot(rvec_ab,rvec_ab);
                    let r_ab = sqrt(r2_ab);
                    // Check if neighbor is within smoothing length
                    let h_ab = 0.5 * (h_a + neighbor.smoothing_length);
                    let h2_ab = h_ab * h_ab;
                    let kernel = kernel_cubic_bspline(r_ab, r2_ab, h_ab, h2_ab);
                    density += neighbor.mass * kernel;
                }
            }
        }
    }
    // Update final density interpolant
    particles[index].density = density;
}


@compute @workgroup_size(256)
fn pressure_equation_of_state(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.num_particles) {
        return;
    }
    // Get particle
    let particle = particles[index];
    let density = particle.density;
    let material_idx = particle.material_idx;
    let rho0 = material[material_idx].density_reference;
    let rho_thresh = material[material_idx].density_reference_threshold;
    let compressibility = material[material_idx].compressibility;
    var pressure = 0.0;
    if density / rho0 >= rho_thresh {
        pressure = compressibility * (density - rho0);
    }

    particles[index].pressure = pressure;

}


@compute @workgroup_size(256)
fn equation_of_motion(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    let num_particles = params.num_particles;
    if (index >= num_particles) {
        return;
    }
    // Get particle
    let particle = particles[index];
    // Get prime values
    let prime = params.grid_prime;
    // Get particle motion
    let motion = particles_motion[index];
    // Get paticle parameters
    let h_a = particle.smoothing_length;
    let mass_a = particle.mass;
    // Initialize Accerleration, (TODO: initialize as disturbance)
    var acceleration = vec3f(0.0,0.0,0.0);
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
                let key = (key_x + key_y + key_z) % num_particles;
                // Find start index in particle list and loop through neihbors
                let idx0 = start_indices[key];
                var spatial_idx: u32 = u32(0);
                for (spatial_idx = idx0; spatial_idx < num_particles; spatial_idx++) {
                    // break if spatial key != particle key
                    // particles[index].material_idx += 1u;
                    if (spatial[spatial_idx].key != key) {
                        break;
                    }
                    let neighbor_idx = spatial[spatial_idx].index;
                    let neighbor = particles[neighbor_idx];
                    let neighbor_motion = particles_motion[neighbor_idx];
                    // Compute distance to neighbor
                    let rvec_ab = get_particle_distance(particle,neighbor,params.grid_size);
                    // let rvec_ab = (vec3f(particle.coord - neighbor.coord) + particle.position - neighbor.position) * params.grid_size;
                    let r2_ab = dot(rvec_ab,rvec_ab);
                    let r_ab = sqrt(r2_ab);
                    // Check if neighbor is within smoothing length
                    let h_ab = 0.5 * (h_a + neighbor.smoothing_length);
                    let h2_ab = h_ab * h_ab;
                    let dkernel_viscosity = dkernel_cubic_bspline(r_ab, r2_ab, h_ab, h2_ab);
                    let dkernel_pressure = dkernel_spiky(r_ab, h_ab, h2_ab);
                    // Calculate Influence from pressure
                    let rho_a = particle.density;
                    let rho_b = neighbor.density;
                    let pressure_a = particle.pressure;
                    let pressure_b = neighbor.pressure;
                    let pressure_on_rho2_delta = pressure_a / (rho_a * rho_a) + pressure_b / (rho_b * rho_b);
                        // Calculate Viscosity using Monaghan & Gingold from 1982
                    let material_a = material[particle.material_idx];
                    let material_b = material[neighbor.material_idx];
                    let vvec_ab = motion.velocity - neighbor_motion.velocity;
                    let v_dot_r_ab = dot(vvec_ab,rvec_ab);
                    var viscosity = 0.0;
                    let cs_ab = 0.5 * (material_a.cs + material_b.cs);
                    let alpha_ab = 0.5 * (material_a.alpha + material_b.alpha);
                    let beta_ab = 0.5 * (material_a.beta + material_b.beta);
                    let eps_ab = 0.5 * (material_a.eps + material_b.eps);
                    let rho_ab = 0.5 * (particle.density + neighbor.density);
                    let eta2 = eps_ab * h2_ab;
                    let nu_ab = h_ab * v_dot_r_ab / (r2_ab + eta2);
                    if (v_dot_r_ab < 0.0 && r2_ab > 1e-8) {
                        viscosity = (-alpha_ab * cs_ab * nu_ab + beta_ab * nu_ab * nu_ab) / rho_ab;
                    }
                    let rhat_ab = rvec_ab / (r_ab + eta2);
                    acceleration += -neighbor.mass * (pressure_on_rho2_delta * dkernel_pressure + viscosity * dkernel_viscosity) * rhat_ab;       
                }
            }
        }
    }
    // Set acceleration
    particles_motion[index].acceleration += acceleration;
}
