fn quadratic_weights(dist: vec3f) -> array<vec3f,3> {
    // Quadratic interpolation weights
    var weight: array<vec3f,3>;
    weight[0] = 0.5 * (0.5 - dist) * (0.5 - dist);
    weight[1] = 0.75 - dist * dist;
    weight[2] = 0.5 * (0.5 + dist) * (05 + dist);
    return weight;
}

fn f32_to_i32(float: f32) -> i32 {
    return i32(float * 1e7);
}

fn i32_to_f32(integer: i32) -> f32 {
    return f32(integer / 1e7);
}

fn get_node_index(coord: vec3f, grid_size: vec3f) -> i32 {
    let index: i32 =
        i32(coord.x) * i32(grid_size.y) * i32(grid_size.z) +
        i32(coord.y) * i32(grid_size.z) +
        i32(coord.z);
    return index;
}
