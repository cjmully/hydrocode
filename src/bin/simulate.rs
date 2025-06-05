use hydrocode::*;
use mls_mpm::*;
use rand::Rng;
use renderer::Renderer;
use winit::event_loop::EventLoop;

fn main() {
    env_logger::init();

    let mut rng = rand::rng();
    let num_particles = 1;
    let dt = 0.01;
    let mass = 1.0;

    let mut particles: Vec<Particle> = vec![];
    let mut materials: Vec<Material> = vec![];
    materials.push(Material {
        eos_density: 1.0,
        eos_threshold: 0.7,
        eos_stiffness: 10.0,
        eos_n: 4.0,
        dynamic_viscosity: 0.1,
        _padding: 0,
    });
    let grid_res: u32 = 64;
    let params = SimParams {
        grid_resolution: grid_res,
        dt,
        scale_distance: 1.0,
        num_particles: num_particles as u32,
        num_nodes: grid_res * grid_res * grid_res,
        _padding: 0,
    };

    let spacing = 0.01;
    // let mut x: f32 = 0.5 - 5.0 * spacing;
    // let mut y: f32 = 0.5 - 5.0 * spacing;
    let mut x: f32 = grid_res as f32 / 2.0 + 0.5;
    let mut y: f32 = grid_res as f32 / 2.0 + 0.5;
    let z: f32 = grid_res as f32 / 2.0 + 0.5;
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
        // let vy = rng.random::<f32>() * 1.0;
        let vy = 10.0;
        let velocity: [f32; 3] = [0.0, vy, 0.0]; // random +y velocity
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
        particles,
        materials,
    };

    let event_loop = EventLoop::new().unwrap();
    let mut renderer = Renderer::default();
    renderer.attach_sim(mls_mpm);
    event_loop.run_app(&mut renderer).unwrap();
}
