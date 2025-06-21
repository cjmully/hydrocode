
@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<storage, read_write> particles_motion: array<ParticleMotion>;

@group(0) @binding(2)
var<storage, read> params: SimParams;

@group(0) @binding(3)
var<uniform> disturbance: Disturbance;

@compute @workgroup_size(64)
fn leap_frog(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
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
    let coord = vec3i(floor(pos + 1e-7));
    let pos_coord_frame = pos - floor(pos + 1e-7); 
    let velocity = 0.5 * (motion.velocity_p + velocity_ph);
    // Set new states
    particles[index].coord = coord;
    particles[index].position = pos_coord_frame;
    particles_motion[index].velocity = velocity;
    particles_motion[index].velocity_p = velocity_ph;
    particles_motion[index].acceleration = vec3f(0.0,0.0,0.0);

    // Boundary check here as cube, change to separate dispacth call
    if (particles[index].coord.x < -10i) {
        particles[index].coord.x = -10i;
        particles[index].position.x = 0.0;
        particles_motion[index].velocity.x *= -0.7;
    }
    if (particles[index].coord.x > 11i) {
        particles[index].coord.x = 11i;
        particles[index].position.x = 0.0;
        particles_motion[index].velocity.x *= -0.7;
    }
    if (particles[index].coord.y < -10i) {
        particles[index].coord.y = -10i;
        particles[index].position.y = 0.0;
        particles_motion[index].velocity.y *= -0.7;
    }
    if (particles[index].coord.y > 11i) {
        particles[index].coord.y = 11i;
        particles[index].position.y = 0.0;
        particles_motion[index].velocity.y *= -0.7;
    }
    if (particles[index].coord.z < -10i) {
        particles[index].coord.z = -10i;
        particles[index].position.z = 0.0;
        particles_motion[index].velocity.z *= -0.7;
    }
    if (particles[index].coord.z > 11i) {
        particles[index].coord.z = 11i;
        particles[index].position.z = 0.0;
        particles_motion[index].velocity.z *= -0.7;
    }
    
}
