struct Particle {
    position: vec3f,
    mass: f32,
    velocity: vec3f,
    material_idx: u32,
    C: mat3x3f, // MLS-MPM Affine Matrix
}
struct Instance {
    position: vec4f,
}
@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> instance: array<vec4f>;

@compute @workgroup_size(256)
fn particle_to_instance(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= arrayLength(&particles)) {
        return;
    }
    let particle = particles[idx];
    // particles coordinate frame is 0,0,0 in lower left corner
    // shift to 0,0,0 in center of screen
    let position = (particle.position) * 1.0; // Multiply by some scale? Should be part of a uniform parameter
    instance[idx] = vec4f(position.x, position.y, position.z, 1.0);
}
