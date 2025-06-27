use csv::WriterBuilder;
use std::fs::OpenOptions;
use std::path::Path;
use serde::Serialize;
use std::error::Error;

use crate::sph::Particle;
use crate::sph::ParticleMotion;

#[derive(Serialize)] // Allow serialization of Particle struct (aka so the code knows how to convert struct into corresponding columns in a CSV file)
struct ParticleData {
    coord_x: i32,
    coord_y: i32,
    coord_z: i32,
    mass: f32,
    position_x: f32,
    position_y: f32,
    position_z: f32,
    density: f32,
    pressure: f32,
    smoothing_length: f32,
    material_idx: u32,
}

#[derive(Serialize)]
struct MotionData {
    vel_x: f32,
    vel_y: f32,
    vel_z: f32,
    drhodt: f32,
    accel_x: f32,
    accel_y: f32,
    accel_z: f32,
    vel_p_x: f32,
    vel_p_y: f32,
    vel_p_z: f32,
}

impl ParticleData {
    fn from_particle(particle: &Particle) -> Self {
        Self {
            coord_x: particle.coord[0],
            coord_y: particle.coord[1],
            coord_z: particle.coord[2],
            mass: particle.mass,
            position_x: particle.position[0],
            position_y: particle.position[1],
            position_z: particle.position[2],
            density: particle.density,
            pressure: particle.pressure,
            smoothing_length: particle.smoothing_length,
            material_idx: particle.material_idx,
        }
    }
}

impl MotionData {
    fn from_motion(motion: &ParticleMotion) -> Self {
        Self {
            vel_x: motion.velocity[0],
            vel_y: motion.velocity[1],
            vel_z: motion.velocity[2],
            drhodt: motion.drho_dt,
            accel_x: motion.acceleration[0],
            accel_y: motion.acceleration[1],
            accel_z: motion.acceleration[2],
            vel_p_x: motion.velocity_p[0],
            vel_p_y: motion.velocity_p[1],
            vel_p_z: motion.velocity_p[2],
        }
    }
}

pub fn write_particles_to_csv(
    file_path_particle: &str,
    particles: &Vec<Particle>,
) -> Result<(), Box<dyn Error>> { // Enum takes 2 arguments so we need the Error
    // Open the file in append mode, create it if it doesn't exist
    let particle_file = OpenOptions::new()
        .write(true)
        .create(true) 
        .truncate(true) // will delete contents in a file if it already exists
        .open(file_path_particle)?;

    // Set up a CSV writer
    let mut particle_writer = WriterBuilder::new().has_headers(false).from_writer(particle_file);

    for particle in particles {
        let particle_data = ParticleData::from_particle(particle);
        particle_writer.serialize(particle_data)?;
    }

    particle_writer.flush()?; // Ensures that all data buffered by the csv writer is immediately written to the underlying file

    Ok(())
}

pub fn write_motion_to_csv(
    file_path_motion: &str,
    motions: &Vec<ParticleMotion>,
) -> Result<(), Box<dyn Error>> {

    let motion_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path_motion)?;
    
    let mut motion_writer = WriterBuilder::new().has_headers(false).from_writer(motion_file);

    for motion in motions {
        let motion_data = MotionData::from_motion(motion);
        motion_writer.serialize(motion_data)?;
    }

    motion_writer.flush()?;

    Ok(())
}