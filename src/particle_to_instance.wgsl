// struct Particle {
//     position: vec3f,
//     mass: f32,
//     velocity: vec3f,
//     material_idx: u32,
//     C: mat3x3f, // MLS-MPM Affine Matrix
// }
// struct Material {
//     color: vec4f,
//     eos_density: f32, // reference density
//     eos_threshold: f32, // negative pressure threshold
//     eos_stiffness: f32, // stiffness coefficient
//     eos_n: f32, // exponent 
//     dynamic_viscosity: f32, // viscosity coefficient
//     rigid_flag: u32,
// }
struct Particle {
    coord: vec3i,
    mass: f32,
    position: vec3f,
    density: f32,
    pressure: f32,
    smoothing_length: f32,
    material_idx: u32,
    _padding: f32,
    // 48 bytes
}
struct Material {
    // Pressure Liquid EOS Parameters
    density_reference: f32,
    density_reference_threshold: f32,
    compressibility: f32,
    boundary_damping: f32,
    // Viscosity Parameters
    cs: f32,
    alpha: f32,
    beta: f32,
    eps: f32,

    color: vec4f,
    // 48 bytes
}
struct RigidParticle {
    coord: vec3i,
    volume: f32,
    position: vec3f,
    body_idx: u32,
    _padding: vec3f,
    smoothing_length: f32,
    // 48 bytes
}
struct RigidBody {
    qbn: vec4f,
    coord: vec3i,
    padding: f32,
    position: vec3f,
    _padding2: f32,
    force: vec3f,
    _padding3: f32,
    torque: vec3f,
    _padding4: f32,
    color: vec4f,
    // 96 bytes
}
struct SimParams{
    grid_prime: vec3u,
    dt: f32,
    grid_size: f32,
    num_particles: u32,
    num_rigid_particles: u32,
    num_total_particles: u32,
    num_rigid_bodies: u32,
    _padding: f32,
    _padding2: f32,
    _padding3: f32,
    // 48 bytes
}
struct Instance {
    position: vec3f,
    color: vec4f,
}
@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<storage, read> materials: array<Material>;
@group(0) @binding(2) var<storage, read> rigid_particles: array<RigidParticle>;
@group(0) @binding(3) var<storage, read> rigid_bodies: array<RigidBody>;
@group(0) @binding(4) var<storage, read> params: SimParams;
@group(0) @binding(5) var<storage, read_write> instance: array<Instance>;

@compute @workgroup_size(256)
fn particle_to_instance(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.num_total_particles) {
        return;
    }
    if (idx < params.num_particles) {
        let particle = particles[idx];
        let position = (vec3f(particle.coord) + particle.position) * params.grid_size;
        instance[idx].position = position;
        instance[idx].color = materials[particle.material_idx].color;
    }
    else if (idx < params.num_particles + params.num_rigid_particles) {
        let rigid_idx = idx - params.num_particles;
        let rigid_particle = rigid_particles[rigid_idx];
        let rigid_body = rigid_bodies[rigid_particle.body_idx];
        let position_body = (vec3f(rigid_body.coord) + rigid_body.position) * params.grid_size;
        let position_particle = (vec3f(rigid_particle.coord) + rigid_particle.position) * params.grid_size;
        instance[idx].position = position_body + position_particle;
        instance[idx].color = vec4f(1.0,1.0,1.0,1.0);
    }
    // let particle = particles[idx];
    // // particles coordinate frame is 0,0,0 in lower left corner
    // // shift to 0,0,0 in center of screen
    // let position = (vec3f(particle.coord) + particle.position) * params.grid_size;
    
    // instance[idx].position = position;
    // // let vel_mag = length(particle.velocity);
    // // let v = clamp(vel_mag,0.0,1.0) / 1.0;
    // // instance[idx].color = vec4f(0.0,0.0,1.0,1.0);
    // instance[idx].color = materials[particle.material_idx].color;
    // // var color = vec4f(0.0);
    // // color = materials[2u].color;
    // // instance[idx].color = color;
}
