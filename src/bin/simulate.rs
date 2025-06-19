use hydrocode::*;
use rand::Rng;
use renderer::Renderer;
use sph::*;
use winit::event_loop::EventLoop;

fn main() {
    env_logger::init();

    let mut rng = rand::rng();
    let num_particles = 10000;
    let dt = 0.01;
    let mass = 0.1;
    let smoothing_length = 0.05;
    let mut particles: Vec<Particle> = vec![];
    let water = Material {
        density_reference: 300.0,
        density_ref_threshold: 0.7,
        compressibility: 0.1,
        boundary_damping: 0.8,
        cs: 5.0,
        alpha: 1.0,
        beta: 2.0,
        eps: 0.01,
    };
    let mut materials = vec![water];
    let mut params = SimParams {
        grid_prime: [59, 519, 1087],
        dt,
        grid_size: 0.1,
        num_particles,
    };
    let disturbance = Disturbance {
        field: [0.0, -9.81, 0.0],
        _padding: 0,
    };

    let spacing = 0.01;
    let init_box_size = 0.5;
    let x_init: f32 = 0.5 - init_box_size / 2.0;
    let z_init: f32 = 0.5 - init_box_size / 2.0;
    let y_init: f32 = 0.6;
    let mut x = x_init;
    let mut y = y_init;
    let mut z = z_init;
    let mut mat_idx = 0;
    for i in 0..num_particles {
        // initialize particles in center of grid
        let position = [x, y, z];
        x += spacing;
        if x > 0.5 + init_box_size / 2.0 {
            x = x_init;
            z += spacing;
            if z > 0.5 + init_box_size / 2.0 {
                z = z_init;
                y += spacing;
            }
        }
        // let vy = rng.random::<f32>() * 1.0;
        let vy = 0.0;
        let velocity: [f32; 3] = [0.0, vy, 0.0]; // random +y velocity
        if i == 50000 {
            mat_idx = 1;
        }
        if i == 75000 {
            mat_idx = 2;
        }
        particles.push(Particle {
            position,
            mass,
            velocity,
            material_idx: mat_idx,
            C: [0.0; 12],
        });
    }
    // Create a wall of rigid particles
    let num_rigid_particles = 20000;
    materials.push(Material {
        color: [1.0, 1.0, 1.0, 0.0],
        eos_density: 1.0,
        eos_threshold: 0.7,
        eos_stiffness: 50.0,
        eos_n: 1.5,
        dynamic_viscosity: 0.1,
        rigid_flag: 1,
        _padding: [0, 0],
    });
    let boundary_min = 1.0 / grid_res as f32;
    let boundary_max = ((grid_res - 2) / grid_res) as f32;
    x = 0.5;
    y = boundary_min;
    z = z_init;
    let spacing = 0.005;
    let mass = 2.0;
    for _i in 0..num_rigid_particles {
        let position = [x, y, z];
        let velocity = [0.0; 3];
        z += spacing;
        if z > 0.5 + init_box_size / 2.0 {
            z = z_init;
            y += spacing;
            if y > 0.5 {
                y = boundary_min;
                x += spacing;
            }
        }
        particles.push(Particle {
            position,
            mass,
            velocity,
            material_idx: 3,
            C: [0.0; 12],
        });
    }
    params.num_particles += num_rigid_particles;
    // Initialize the MLS-MPM Compute Shaders
    let mls_mpm = MlsMpm {
        params,
        disturbance,
        particles,
        materials,
    };

    let event_loop = EventLoop::new().unwrap();
    let mut renderer = Renderer::default();
    renderer.attach_sim(mls_mpm);
    event_loop.run_app(&mut renderer).unwrap();
}
