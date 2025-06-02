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
@group(1) @binding(1) var<storage, read_write> instance: array<vec4f>;

@compute @workgroup_size(256)
fn particle_to_instance(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.num_particles) {
        return;
    }
    let particle = particles[idx];
    let position = particle.position;
    instance[idx] = vec4f(position.x, position.y, position.z, 1.0);
}
