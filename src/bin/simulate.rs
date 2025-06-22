use hydrocode::*;
use renderer::Renderer;
use sph::*;
use winit::event_loop::EventLoop;

fn main() {
    env_logger::init();

    let num_particles = 20000;
    let dt = 0.001;
    let mass = 0.1;
    let smoothing_length = 0.05;
    let mut particles: Vec<Particle> = vec![];
    let mut motion: Vec<ParticleMotion> = vec![];
    let water = Material {
        density_reference: 200.0,
        density_ref_threshold: 0.7,
        compressibility: 0.1,
        boundary_damping: 0.8,
        cs: 5.0,
        alpha: 1.0,
        beta: 2.0,
        eps: 0.01,
        color: [0.0, 0.0, 1.0, 1.0],
    };
    let custom = Material {
        density_reference: 200.0,
        density_ref_threshold: 0.7,
        compressibility: 0.1,
        boundary_damping: 0.8,
        cs: 5.0,
        alpha: 1.0,
        beta: 2.0,
        eps: 0.01,
        color: [1.0, 1.0, 1.0, 1.0],
    };
    let materials = vec![water, custom];
    let params = SimParams {
        grid_prime: [59, 519, 1087],
        dt,
        grid_size: 0.1,
        num_particles,
        _padding: [0.0; 2],
    };
    let disturbance = Disturbance {
        field: [0.0, 0.0, 0.0],
        _padding: 0.0,
    };

    let spacing = 0.01;
    let init_box_size = 0.8;
    let x_init: f32 = 0.0 - init_box_size / 2.0;
    let z_init: f32 = 0.0 - init_box_size / 2.0;
    let y_init: f32 = 0.0 - init_box_size / 2.0;
    let mut x = x_init;
    let mut y = y_init;
    let mut z = z_init;
    let velocity: [f32; 3] = [0.0; 3];
    let acceleration: [f32; 3] = [0.0; 3];
    let density: f32 = 0.0;
    let pressure: f32 = 0.0;
    let mut material_idx = 0;
    for i in 0..num_particles {
        // initialize particles in center of grid
        let coord_x = (x / params.grid_size).floor();
        let coord_y = (y / params.grid_size).floor();
        let coord_z = (z / params.grid_size).floor();
        let pos_x = x / params.grid_size - coord_x;
        let pos_y = y / params.grid_size - coord_y;
        let pos_z = z / params.grid_size - coord_z;
        let position = [pos_x, pos_y, pos_z];
        let coord = [coord_x as i32, coord_y as i32, coord_z as i32];
        if i >= 10000 {
            material_idx = 1;
        }
        particles.push(Particle {
            coord,
            position,
            mass,
            density,
            pressure,
            material_idx,
            smoothing_length,
            _padding: 0.0,
        });
        motion.push(ParticleMotion {
            velocity,
            drho_dt: 0.0,
            acceleration,
            _padding: 0.0,
            velocity_p: velocity,
            _padding2: 0.0,
        });
        x += spacing;
        if x >= init_box_size / 2.0 {
            x = x_init;
            z += spacing;
            if z >= init_box_size / 2.0 {
                z = z_init;
                y += spacing;
            }
        }
    }
    // Initialize the MLS-MPM Compute Shaders
    let sph = Sph {
        params,
        motion,
        disturbance,
        particles,
        materials,
    };
    println!("num particles {:?}", params.num_particles);

    let event_loop = EventLoop::new().unwrap();
    let mut renderer = Renderer::default();
    renderer.attach_sim(sph);
    event_loop.run_app(&mut renderer).unwrap();
}
