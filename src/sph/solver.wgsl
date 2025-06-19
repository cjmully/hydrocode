
@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<storage, read_write> particles_motion: array<ParticleMotion>;

@group(1) @binding(0)
var<uniform> simulation: Simulation;

@group(1) @binding(1)
var<storage, read> grid_param: GridParam;

@compute @workgroup_size(64)
fn leap_frog(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    let array_length = arrayLength(&particles);
    if (index >= array_length) {
        return;
    }
    // Get particle
    let particle = particles[index];
    let motion = particles_motion[index];
    // Get new coord position & velocity
    let velocity_ph = motion.velocity_p + motion.acceleration * simulation.dt + simulation.disturbance * simulation.dt;
    let pos = vec3f(particle.coord) + particle.position + velocity_ph * simulation.dt / grid_param.grid_size;
    let coord = vec3i(floor(pos + 1e-7));
    let pos_coord_frame = pos - floor(pos + 1e-7); 
    let velocity = 0.5 * (motion.velocity_p + velocity_ph);
    // Set new states
    particles[index].coord = coord;
    particles[index].position = pos_coord_frame;
    particles_motion[index].velocity = velocity;
    particles_motion[index].velocity_p = velocity_ph;
    particles_motion[index].acceleration = vec3f(0.0,0.0,0.0);
    
}
