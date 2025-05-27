use hydrocode::*;
use mls_mpm::*;
use rand::Rng;

fn main() {
    let mut rng = rand::rng();
    let num_particles = 100;
    let dt = 0.01;
    let mass = 0.001;

    let mut particles: Vec<Particle> = vec![];
    let params = SimParams {
        grid_resolution: 5,
        dt,
        scale_distance: 1.0,
        num_particles: num_particles as u32,
        num_nodes: 5 * 5 * 5,
        _padding: 0,
    };

    let spacing = 0.01;
    let mut x: f32 = 0.5 - 5.0 * spacing;
    let mut y: f32 = 0.5 - 5.0 * spacing;
    let z: f32 = 0.5;
    let mut row = 1;
    for _i in 0..num_particles {
        // initialize particles in center of grid
        let position = [x, y, z];
        x += spacing;
        if x > 0.5 + 5.0 * spacing {
            x = 0.5 - 5.0 * spacing;
            if row % 2 != 0 {
                x += spacing; // This staggars the rows a bit
            }
            y += spacing;
            row += 1;
        }
        let vy = rng.random::<f32>();
        let velocity: [f32; 3] = [0.0, vy.abs() / 10.0, 0.0]; // random +y velocity
        particles.push(Particle {
            position,
            mass,
            velocity,
            _padding: 0,
            C: [0.0; 12],
        });
    }
    // Initialize the MLS-MPM Compute Shaders
    let mls_mpm = pollster::block_on(MlsMpm::new(params));

    // Map to buffers
    mls_mpm.cpu2gpu_particles(&particles);
    mls_mpm.cpu2gpu_params(&params);

    // Run Compute Shader
    mls_mpm.compute_particle_to_grid();
    let particles_out = mls_mpm.gpu2cpu_particles();

    // Print output
    println!("Pre Pass");
    for i in 0..10 {
        println!(
            "Position: {:?}, Velocity: {:?}, Mass: {:?}",
            particles_out[i].position, particles_out[i].velocity, particles_out[i].mass
        );
    }

    let grid_out = mls_mpm.gpu2cpu_grid();
    for i in 0..params.num_nodes as usize {
        println!("{:?}", grid_out[i]);
    }

    mls_mpm.compute_grid_to_particle();

    // Get Output
    let particles_out = mls_mpm.gpu2cpu_particles();

    // Print output
    println!("Post Pass");
    for i in 0..10 {
        println!(
            "Position: {:?}, Velocity: {:?}, Mass: {:?}",
            particles_out[i].position, particles_out[i].velocity, particles_out[i].mass
        );
    }
}
