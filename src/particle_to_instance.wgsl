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
struct SimParams{
    grid_prime: vec3u,
    dt: f32,
    grid_size: f32,
    num_particles: u32,
    _padding: vec2f,
    // 32 bytes
}
struct Instance {
    position: vec3f,
    color: vec4f,
}
@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<storage, read> materials: array<Material>;
@group(0) @binding(2) var<storage, read> params: SimParams;
@group(0) @binding(3) var<storage, read_write> instance: array<Instance>;

@compute @workgroup_size(256)
fn particle_to_instance(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= arrayLength(&particles)) {
        return;
    }
    let particle = particles[idx];
    // particles coordinate frame is 0,0,0 in lower left corner
    // shift to 0,0,0 in center of screen
    let position = (vec3f(particle.coord) + particle.position) * params.grid_size;
    
    instance[idx].position = position;
    // let vel_mag = length(particle.velocity);
    // let v = clamp(vel_mag,0.0,1.0) / 1.0;
    // instance[idx].color = vec4f(0.0,0.0,1.0,1.0);
    instance[idx].color = materials[particle.material_idx].color;
    // var color = vec4f(0.0);
    // color = materials[2u].color;
    // instance[idx].color = color;
}
