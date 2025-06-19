fn quadratic_weights(dist: vec3f) -> array<vec3f,3> {
    // Quadratic interpolation weights
    var weight: array<vec3f,3>;
    weight[0] = 0.5 * (0.5 - dist) * (0.5 - dist);
    weight[1] = 0.75 - dist * dist;
    weight[2] = 0.5 * (0.5 + dist) * (0.5 + dist);
    return weight;
}

fn f32_to_i32(float: f32) -> i32 {
    return i32(clamp(float * 1.0e5, -2.0e9, 2.0e9));
}

fn i32_to_f32(integer: i32) -> f32 {
    return f32(integer) * 1.0e-5;
}

fn get_node_index(coord: vec3f, grid_resolution: u32) -> u32 {
    let res = i32(grid_resolution);
    let x = clamp(i32(coord.x), 0i, res - 1i);
    let y = clamp(i32(coord.y), 0i, res - 1i);
    let z = clamp(i32(coord.z), 0i, res - 1i);
    return u32(x * res * res + y * res + z);
}

