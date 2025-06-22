
@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<storage, read_write> particles_motion: array<ParticleMotion>;

@group(0) @binding(2)
var<storage, read> params: SimParams;

@group(0) @binding(3)
var<uniform> disturbance: Disturbance;

@compute @workgroup_size(256)
fn leap_frog(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    let num_particles = params.num_particles;
    if (index >= params.num_particles) {
        return;
    }
    // Get particle
    let particle = particles[index];
    // Get particle motion
    let motion = particles_motion[index];
    // Get timestep
    let dt = params.dt;
    // Get new coord position & velocity
    let velocity_ph = motion.velocity_p + motion.acceleration * dt + disturbance.field * dt;
    let pos = vec3f(particle.coord) + particle.position + velocity_ph * dt / params.grid_size;
    let coord = vec3i(floor(pos));
    let pos_coord_frame = pos - floor(pos); 
    let velocity = 0.5 * (motion.velocity_p + velocity_ph);
    // Set new states
    particles[index].coord = coord;
    particles[index].position = pos_coord_frame;
    particles_motion[index].velocity = velocity;
    particles_motion[index].velocity_p = velocity_ph;
    particles_motion[index].acceleration = vec3f(0.0,0.0,0.0);

    // Boundary check here as cube, change to separate dispacth call
    var position = pos * params.grid_size;
    let boundary_damping = 0.7;
    let bounds: f32 = 0.5;
    if (abs(position.x) > bounds && sign(position.x) == sign(motion.velocity_p.x)) {
        let velocity = -1.0 * motion.velocity_p.x * boundary_damping;
        position.x += sign(position.x) * (abs(position.x) - bounds) * (1.0 + boundary_damping);
        particles_motion[index].velocity.x = velocity;
        particles_motion[index].velocity_p.x = velocity;
        particles[index].coord.x = i32(floor(sign(position.x) * bounds / params.grid_size));
        particles[index].position.x = (sign(position.x) * bounds / params.grid_size - floor(sign(position.x) * bounds / params.grid_size));
    }

    if (abs(position.y) > bounds && sign(position.y) == sign(motion.velocity_p.y)) {
        let velocity = -1.0 * motion.velocity_p.y * boundary_damping;
        position.y += sign(position.y) * (abs(position.y) - bounds) * (1.0 + boundary_damping);
        particles_motion[index].velocity.y = velocity;
        particles_motion[index].velocity_p.y = velocity;
        particles[index].coord.y = i32(floor(sign(position.y) * bounds / params.grid_size));
        particles[index].position.y = (sign(position.x) * bounds / params.grid_size - floor(sign(position.x) * bounds / params.grid_size));
    }
    
    if (abs(position.z) > bounds && sign(position.z) == sign(motion.velocity_p.z)) {
        let velocity = -1.0 * motion.velocity_p.z * boundary_damping;
        position.z += sign(position.z) * (abs(position.z) - bounds) * (1.0 + boundary_damping);
        particles_motion[index].velocity.z = velocity;
        particles_motion[index].velocity_p.z = velocity;
        particles[index].coord.z = i32(floor(sign(position.z) * bounds / params.grid_size));
        particles[index].position.z = (sign(position.x) * bounds / params.grid_size - floor(sign(position.x) * bounds / params.grid_size));
    }    
}
