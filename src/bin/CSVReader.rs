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

pub fn read_csv() -> io::Result<Vec<Data42>> {
    let path = r"C:\Users\fbfusco\42Software\hwo-acs-20-francesca-fusco\42\InOut\RustOutputParameters.42";
    let file = File::open(&path)?;
    let reader = io::BufReader::new(file); //https://doc.rust-lang.org/std/io/trait.BufRead.html

    let mut data = Vec::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = line?; // unwrap the line
        let values: Vec<f32> = line
            .split_whitespace() // split line into string slices
            //.split(',')
            .filter_map(|s| s.parse::<f32>().ok())
            //.filter_map(|s| s.trim().parse::<f32>().ok())
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