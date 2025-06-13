use hydrocode::*;
use mls_mpm::*;
use rand::Rng;
use renderer::Renderer;
use winit::event_loop::EventLoop;

fn main() {
    env_logger::init();

    let mut rng = rand::rng();
    let num_particles = 50000;
    let dt = 0.01;
    let mass = 0.1;

    let mut particles: Vec<Particle> = vec![];
    let mut materials: Vec<Material> = vec![];
    materials.push(Material {
        color: [0.0, 0.0, 1.0, 0.0],
        eos_density: 1.0,
        eos_threshold: 0.1,
        eos_stiffness: 50.0,
        eos_n: 1.2,
        dynamic_viscosity: 1.0,
        _padding: 0,
    });
    materials.push(Material {
        color: [0.0, 1.0, 0.0, 0.0],
        eos_density: 3.0,
        eos_threshold: 0.1,
        eos_stiffness: 50.0,
        eos_n: 1.0,
        dynamic_viscosity: 1.0,
        _padding: 0,
    });
    let grid_res: u32 = 32;
    let params = SimParams {
        grid_resolution: grid_res,
        dt,
        scale_distance: 1.0,
        num_particles: num_particles as u32,
        num_nodes: grid_res * grid_res * grid_res,
        _padding: 0,
    };
    let disturbance = Disturbance {
        field: [0.0, -9.81, 0.0],
        _padding: 0,
    };

    let spacing = 0.01;
    let init_box_size = 0.5;
    let x_init: f32 = 0.5 - init_box_size / 2.0;
    let z_init: f32 = 0.5 - init_box_size / 2.0;
    let y_init: f32 = 0.2;
    let mut x = x_init;
    let mut y = y_init;
    let mut z = z_init;
    let mut mat_idx = 2;
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
        if i > 40000 {
            mat_idx = 1;
        }
        particles.push(Particle {
            position,
            mass,
            velocity,
            material_idx: 0,
            C: [0.0; 12],
        });
    }

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
