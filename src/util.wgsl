fn quadratic_weights(dist: vec3f) -> array<vec3f,3> {
    // Quadratic interpolation weights
    var weight: array<vec3f,3>;
    weight[0] = 0.5 * (0.5 - dist) * (0.5 - dist);
    weight[1] = 0.75 - dist * dist;
    weight[2] = 0.5 * (0.5 + dist) * (0.5 + dist);
    return weight;
}

fn f32_to_i32(float: f32) -> i32 {
    return i32(float * 1.0e7);
}

fn i32_to_f32(integer: i32) -> f32 {
    return f32(integer) * 1.0e-7;
}

fn get_node_index(coord: vec3f, grid_resolution: u32) -> u32 {
    let index: u32 =
        u32(max(coord.x,0.0)) * u32(grid_resolution) * u32(grid_resolution) +
        u32(max(coord.y,0.0)) * u32(grid_resolution) +
        u32(max(coord.z,0.0));
    return index;
}
