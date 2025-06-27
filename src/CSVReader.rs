use std::fs::File;
use std::io::{self, BufRead}; // if you want to read by line, you’ll need BufRead
use std::path::Path;

pub struct Data42 {
    pub body_rates: [f32; 3],
    pub angular_accel: [f32; 3],
    pub linear_accel: [f32; 3],
    pub simtime: [f32;1],
    pub quaternion: [f32; 4]
}

pub struct CSVParticleData {
    pub coordinates: [i32; 3],
    pub particle_mass: f32,
    pub positions: [f32; 3],
    pub particle_density: f32,
    pub particle_pressure: f32,
    pub particle_smoothing_length: f32,
    pub particle_material_idx: u32,
}

pub struct CSVMotionData {
    pub particle_velocity: [f32;3],
    pub particle_drho_dt: f32,
    pub particle_acceleration: [f32;3],
    pub particle_velocity_p: [f32;3],
}

pub fn read_csv() -> io::Result<Vec<Data42>> {
    let path = r"C:\Users\fbfusco\42Software\hwo-acs-20-francesca-fusco\42\InOut\RustOutputParameters.42";
    //let path = r"C:\Users\fbfusco\FakeCSVDataforRust.csv";
    let file = File::open(&path)?;
    let reader = io::BufReader::new(file); //https://doc.rust-lang.org/std/io/trait.BufRead.html

    let mut data = Vec::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = line?; // unwrap the line
        let values: Vec<f32> = line
            .split_whitespace() // split line into string slices
            //.split(',') // fake CSV
            .filter_map(|s| s.parse::<f32>().ok())
            //.filter_map(|s| s.trim().parse::<f32>().ok()) // fake CSV
            .collect(); 


        let simtime = [values[0]];
        let body_rates = [values[1], values[2], values[3]];
        let angular_accel = [values[4], values[5], values[6]];
        let linear_accel = [values[7], values[8], values[9]];
        let quaternion = [values[10], values[11], values[12], values[13]];

        data.push(Data42 {
            simtime,
            body_rates,
            angular_accel,
            linear_accel,
            quaternion,
        });
    }

    Ok(data)
}

pub fn read_particle_csv() -> io::Result<Vec<CSVParticleData>> {
    let read_particle_path = r"C:\Users\fbfusco\particle_data.csv";
    let read_particle_file = File::open(&read_particle_path)?;
    let read_particle_reader = io::BufReader::new(read_particle_file);
    let mut read_particle_data = Vec::new();

    for (particle_line_num, particle_line) in read_particle_reader.lines().enumerate() {
        let particle_line = particle_line?;
        let particle_values: Vec<f32> = particle_line
            .split(',')
            .filter_map(|s| s.trim().parse::<f32>().ok())
            .collect();
        
        let coordinates = [particle_values[0] as i32, particle_values[1] as i32, particle_values[2] as i32];
        let particle_mass = particle_values[3];
        let positions = [particle_values[4], particle_values[5], particle_values[6]];
        let particle_density = particle_values[7];
        let particle_pressure = particle_values[8];
        let particle_smoothing_length = particle_values[9];
        let particle_material_idx = particle_values[10] as u32;

        read_particle_data.push(CSVParticleData {
            coordinates,
            particle_mass,
            positions,
            particle_density,
            particle_pressure,
            particle_smoothing_length,
            particle_material_idx,
        });
    }
    Ok(read_particle_data)
}

pub fn read_motion_csv() -> io::Result<Vec<CSVMotionData>> {
    let read_motion_path = r"C:\Users\fbfusco\motion_data.csv";
    let read_motion_file = File::open(&read_motion_path)?;
    let read_motion_reader = io::BufReader::new(read_motion_file);
    let mut read_motion_data = Vec::new();

    for (motion_line_num, motion_line) in read_motion_reader.lines().enumerate() {
        let motion_line = motion_line?;
        let motion_values: Vec<f32> = motion_line
            .split(',')
            .filter_map(|s| s.trim().parse::<f32>().ok())
            .collect();
        
        let particle_velocity = [motion_values[0], motion_values[1], motion_values[2]];
        let particle_drho_dt = motion_values[3];
        let particle_acceleration = [motion_values[4], motion_values[5], motion_values[6]];
        let particle_velocity_p = [motion_values[7], motion_values[8], motion_values[9]];

        read_motion_data.push(CSVMotionData {
            particle_velocity,
            particle_drho_dt,
            particle_acceleration,
            particle_velocity_p,
        });
    }
    Ok(read_motion_data)
}