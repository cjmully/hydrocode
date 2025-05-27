struct Grid {
    vx: i32,
    vy: i32,
    vz: i32,
    mass: i32,
}

@group(0) @binding(0) var<storage, read_write> grid: array<Grid>;

@compute @workgroup_size(256)
fn grid_reset(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx < arrayLength(&grid)) {
        var node = grid[idx];
        node.vx = 0;
        node.vy = 0;
        node.vz = 0;
        node.mass = 0;

        grid[idx] = node;
    }
}
