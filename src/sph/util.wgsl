// WGSL file for common SPH supporting functions

// get distance from particle to neighbor
fn get_particle_distance(particle: Particle, neighbor: Particle, scale: f32) -> vec3f {
    let coord_dist_x = f32(particle.coord.x - neighbor.coord.x);
    let coord_dist_y = f32(particle.coord.y - neighbor.coord.y);
    let coord_dist_z = f32(particle.coord.z - neighbor.coord.z);
    let rvec_ab = (vec3f(coord_dist_x, coord_dist_y, coord_dist_z) + particle.position - neighbor.position) * scale;
    return rvec_ab;
}
fn get_rigid_particle_distance(particle: RigidParticle, neighbor: RigidParticle, scale: f32) -> vec3f {
    let coord_dist_x = f32(particle.coord.x - neighbor.coord.x);
    let coord_dist_y = f32(particle.coord.y - neighbor.coord.y);
    let coord_dist_z = f32(particle.coord.z - neighbor.coord.z);
    let rvec_ab = (vec3f(coord_dist_x, coord_dist_y, coord_dist_z) + particle.position - neighbor.position) * scale;
    return rvec_ab;
}
fn get_particle_to_rigid_distance(particle: Particle, neighbor: RigidParticle, scale: f32) -> vec3f {
    let coord_dist_x = f32(particle.coord.x - neighbor.coord.x);
    let coord_dist_y = f32(particle.coord.y - neighbor.coord.y);
    let coord_dist_z = f32(particle.coord.z - neighbor.coord.z);
    let rvec_ab = (vec3f(coord_dist_x, coord_dist_y, coord_dist_z) + particle.position - neighbor.position) * scale;
    return rvec_ab;
}

// get starting index of particles in selected grid coordinate
fn get_coord_hash_key(grid_coord: vec3i, array_length: u32) -> u32 {
        let key_x = u32(grid_coord.x) * params.grid_prime.x;
        let key_y = u32(grid_coord.y) * params.grid_prime.y;
        let key_z = u32(grid_coord.z) * params.grid_prime.z;
        let key = (key_x + key_y + key_z) % array_length;
        return key;
} 
