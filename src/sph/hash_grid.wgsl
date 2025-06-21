@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<storage, read_write> spatial_scattered: array<SpatialLookup>;

@group(0) @binding(2)
var<storage, read_write> start_indices: array<u32>;

@group(0) @binding(3)
var<storage, read> params: SimParams;

@compute @workgroup_size(64)
fn spatial_lookup(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.num_particles) {
        return;
    }
    // Get particle
    let particle = particles[index];
    // Get grid coordinates
    let grid_coord = particle.coord;
    // Get prime values
    let prime = params.grid_prime;
    // Calculate hash key components
    // WGSL addition wraps around integers
    // This is the appropriate behavior for our hash key
    let key_x = u32(grid_coord.x) * prime.x;
    let key_y = u32(grid_coord.y) * prime.y;
    let key_z = u32(grid_coord.z) * prime.z;
    
    // Combine for final hash key (with wrapping)
    let key = key_x + key_y + key_z;
    
    // Divide hash key by array length using % remainder function
    spatial_scattered[index].key = key % params.num_particles;
    spatial_scattered[index].index = index;
    // Set start indices at max value
    start_indices[index] = U32MAX;
    
}
