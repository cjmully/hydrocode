// WGSL file for common SPH supporting functions

// get distance from particle to neighbor
fn get_particle_distance(particle: Particle, neighbor: Particle, scale: f32) -> vec3f {
    let rvec_ab = (vec3f(particle.coord - neighbor.coord) + particle.position - neighbor.position) * scale;
    return rvec_ab;
}

// get starting index of particles in selected grid coordinate
fn get_coord_hash_key(grid_coord: vec3i, array_length: u32) -> u32 {
        let key_x = u32(grid_coord.x) * grid_param.prime.x;
        let key_y = u32(grid_coord.y) * grid_param.prime.y;
        let key_z = u32(grid_coord.z) * grid_param.prime.z;
        let key = (key_x + key_y + key_z) % array_length;
        return key;
} 
