struct Grid {
    vx: i32,
    vy: i32,
    vz: i32,
    mass: i32,
}

struct SimParams {
    grid_resolution: u32,
    dt: f32,
    scale_distance: f32,
    num_particles: u32,
    num_nodes: u32,
    _padding: u32,
}

@group(0) @binding(0) var<storage, read_write> grid: array<Grid>;
@group(0) @binding(1) var<uniform> params: SimParams;

@compute @workgroup_size(256)
fn grid_update(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.num_nodes) {
        return;
    }
    let node = grid[idx];
    let grid_res = params.grid_resolution;

    // Convert momentum to velocity
    if (node.mass > 0) {
        var velocity: vec3f = vec3f(i32_to_f32(node.vx), i32_to_f32(node.vy), i32_to_f32(node.vz));
        velocity /= i32_to_f32(node.mass);
        grid[idx].vx = f32_to_i32(velocity.x);
        grid[idx].vy = f32_to_i32(velocity.y);
        grid[idx].vz = f32_to_i32(velocity.z);
        
        let x = idx / grid_res / grid_res;
        let y = idx / grid_res % grid_res;
        let z = idx % grid_res;
        if (x < 2 || x > grid_res - 3) { grid[idx].vx = 0; }
        if (y < 2 || y > grid_res - 3) { grid[idx].vy = 0; }
        if (z < 2 || z > grid_res - 3) { grid[idx].vz = 0; }
    }
}
