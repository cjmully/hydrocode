use hydrocode::*;
use mls_mpm::*;
use renderer::Renderer;
use std::f32::consts::PI;
use winit::event_loop::EventLoop;

fn main() {
    env_logger::init();

    let dt = 0.01;
    let mut num_particles = 0;
    let mut particles: Vec<Particle> = vec![];

    // Simulation parameters
    let grid_res: u32 = 32;
    let mut params = SimParams {
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

    let water = Material {
        color: [0.0, 0.0, 1.0, 0.0],
        eos_density: 6.0,
        eos_threshold: 0.7,
        eos_stiffness: 50.0,
        eos_n: 1.5,
        dynamic_viscosity: 0.1,
        rigid_flag: 0,
        _padding: [0, 0],
    };
    let mut materials = vec![water];
    // Initialize propellant inside tank
    let radius: f32 = 0.3;
    let latitude_segments = 100;
    let longitude_segments = 100;
    let mass = 1.0;
    // 200 x 200 = 40000 particles
    for lat in 1..latitude_segments {
        let theta = lat as f32 * PI / latitude_segments as f32;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        for lon in 0..longitude_segments {
            // Spherical to cartesian coordinates
            let phi = lon as f32 * 2.0 * PI / longitude_segments as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = sin_theta * cos_phi;
            let y = cos_theta;
            let z = sin_theta * sin_phi;
            let position = [x * radius + 0.5, y * radius + 0.5, z * radius + 0.5];
            let velocity: [f32; 3] = [0.0; 3];
            particles.push(Particle {
                position,
                mass,
                velocity,
                material_idx: 0,
                C: [0.0; 12],
            });
            num_particles += 1;
        }
    }
    params.num_particles += num_particles;

    // Spherical fuel tank with rigid particles
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
    let radius: f32 = 0.4;
    let latitude_segments = 300;
    let longitude_segments = 300;
    let mass = 1.0;
    let mut num_rigid_particles = 0;
    // 200 x 200 = 10000 particles
    for lat in 1..latitude_segments {
        let theta = lat as f32 * PI / latitude_segments as f32;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        for lon in 0..longitude_segments {
            // Spherical to cartesian coordinates
            let phi = lon as f32 * 2.0 * PI / longitude_segments as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = sin_theta * cos_phi;
            let y = cos_theta;
            let z = sin_theta * sin_phi;
            let position = [x * radius + 0.5, y * radius + 0.5, z * radius + 0.5];
            let velocity: [f32; 3] = [0.0; 3];
            particles.push(Particle {
                position,
                mass,
                velocity,
                material_idx: 1,
                C: [0.0; 12],
            });
            num_rigid_particles += 1;
        }
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
