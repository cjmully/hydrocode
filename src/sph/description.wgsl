
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
struct ParticleMotion {
    velocity: vec3f,
    drho_dt: f32,
    acceleration: vec3f,
    _padding: f32,
    velocity_p: vec3f,
    _padding2: f32,
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
struct SpatialLookup {
    index: u32,
    key: u32,
    // 8 bytes
}
struct GridParam {
    prime: vec3u,
    grid_size: f32,
    // 16 bytes
}
struct SimParams{
    grid_prime: vec3u,
    dt: f32,
    grid_size: f32,
    num_particles: u32,
    _padding: vec2f,
    // 32 bytes
}
struct Disturbance {
    local_position: vec3f,
    _padding: f32,
    local_velocity: vec3f,
    _padding2: f32,
    body_rates: vec3f,
    _padding3: f32,
    angular_accel: vec3f,
    _padding4: f32,
    linear_accel: vec3f,
    _padding5: f32,
    simtime: f32,
    _padding6: vec3f,
    // 16 bytes
}
const DIMENSION: u32 = 9u; // 9u = 2 dimesion, 27u = 3 dimension
const U32MAX: u32 = 4294967295u;
const PI: f32 = 3.1415927;
