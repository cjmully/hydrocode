use hydrocode::*;
use renderer::Renderer;
use sph::*;
use winit::event_loop::EventLoop;
use CSVReader::{read_csv, Data42, read_particle_csv, CSVParticleData, read_motion_csv, CSVMotionData};

fn main() {
    env_logger::init();

    let num_particles = 20000;
    let dt = 0.001;
    let mass = 0.1;
    let smoothing_length = 0.05;
    let mut particles: Vec<Particle> = vec![];
    let mut motion: Vec<ParticleMotion> = vec![];
    let mut disturbance: Vec<Disturbance> = vec![];
    let water = Material {
        density_reference: 2000.0,
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
        density_reference: 2000.0,
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


   //let mut disturbance = Disturbance {
   //     local_position: [0.5, 0.5, 0.0],
   //    _padding: 0.0,
   //     local_velocity: [0.0, 0.0, 0.0],
   //     _padding2: 0.0,
   //     body_rates: [0.0, 0.0, 0.0], // Default values
   //     _padding3: 0.0,
   //     angular_accel: [0.0, 0.0, 0.0],
   //     _padding4: 0.0,
   //     linear_accel: [0.0, 0.0, 0.0],
   //     _padding5: 0.0,
   //     simtime: 0.0,
   //     _padding6: [0.0;7],
   //};

    let  csv_data = read_csv().unwrap();

    for (step_count, data) in csv_data.iter().enumerate() {

        disturbance.push(Disturbance { 
            local_position: [0.0, 1.0, 0.0],
            _padding: 0.0,
            body_rates: data.body_rates,
            _padding2: 0.0,
            angular_accel: data.angular_accel,
            _padding3: 0.0,
            linear_accel: data.linear_accel,
            _padding4: 0.0,
            simtime: data.simtime[0],
            _padding5: [0.0; 7],
        })
    }

    let particle_csv_data = read_particle_csv().unwrap();

    // for (i, read_particle_data) in particle_csv_data.iter().enumerate().take(5) { // only first 5 lines
    //     println!("Line Number {}", i + 1);
    //     println!(
    //         "Coordinates [{}, {}, {}]", 
    //         read_particle_data.coordinates[0], 
    //         read_particle_data.coordinates[1], 
    //         read_particle_data.coordinates[2]
    //     );
    // }

    let motion_csv_data = read_motion_csv().unwrap();

    // for(i, read_motion_data) in motion_csv_data.iter().enumerate().take(5) { // only first 5 lines
    //     println!("Line Number {}", i +1);
    //     println!(
    //         "Velocity [{}, {}, {}]",
    //         read_motion_data.particle_velocity[0],
    //         read_motion_data.particle_velocity[1],
    //         read_motion_data.particle_velocity[2]
    //     );
    // }


    let spacing = 0.02;
    let init_box_size = 0.8;
    let x_init: f32 = 0.0 - init_box_size / 2.0;
    let z_init: f32 = 0.0 - init_box_size / 2.0;
    let y_init: f32 = 0.0 - 5.0 * spacing;
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
        // let coord_x = (x / params.grid_size).floor();
        // let coord_y = (y / params.grid_size).floor();
        // let coord_z = (z / params.grid_size).floor();
        // let pos_x = x / params.grid_size - coord_x;
        // let pos_y = y / params.grid_size - coord_y;
        // let pos_z = z / params.grid_size - coord_z;
        // let position = [pos_x, pos_y, pos_z];
        // let coord = [coord_x as i32, coord_y as i32, coord_z as i32];
        // if i >= 10000 {
        //     material_idx = 1;
        // }

        // Take in CSV data as our initial states
        particles.push(Particle {
            coord: particle_csv_data[i as usize].coordinates,
            position: particle_csv_data[i as usize].positions,
            mass: particle_csv_data[i as usize].particle_mass,
            density: particle_csv_data[i as usize].particle_density,
            pressure: particle_csv_data[i as usize].particle_pressure,
            material_idx: particle_csv_data[i as usize].particle_material_idx,
            smoothing_length: particle_csv_data[i as usize].particle_smoothing_length,
            _padding: 0.0,
        });

        motion.push(ParticleMotion {
            velocity: motion_csv_data[i as usize].particle_velocity,
            drho_dt: motion_csv_data[i as usize].particle_drho_dt,
            acceleration: motion_csv_data[i as usize].particle_acceleration,
            _padding: 0.0,
            velocity_p: motion_csv_data[i as usize].particle_velocity_p,
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
    println!("num particles {:?}", &params.num_particles);
    // Initialize the MLS-MPM Compute Shaders
    let sph = Sph::new(params, disturbance, particles, motion, materials);

    let event_loop = EventLoop::new().unwrap();
    let mut renderer = Renderer::default();
    renderer.attach_sim(sph);
    event_loop.run_app(&mut renderer).unwrap();
}
