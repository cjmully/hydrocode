// WGSL File for all Kernel and Kernel derivative functions

// Cubic B-Spline 
fn kernel_cubic_bspline(r: f32, r2: f32, h: f32, h2: f32) -> f32 {
    let kernel_normalization = h2 * h * PI / 8.0; // 3-D normalization
    // let kernel_normalization = h2 * 7.0 * PI / 40.0; // 2-D normalization
    let k_rh = 1.0 - r / h;
    let k_hi = 2.0 * k_rh * k_rh * k_rh;
    let k_lo = 6.0 * (r2 * r/(h2 * h)) - 6.0 * (r2 / h2) + 1.0;
    var kernel = 0.0;
    if (r < 0.5 * h) {
        kernel = k_lo / kernel_normalization;
    } else if (r <= h) {
        kernel = k_hi / kernel_normalization;
    }
    return kernel;
} 
// Cubic B-Spline Derivative
fn dkernel_cubic_bspline(r: f32, r2: f32, h: f32, h2: f32) -> f32 {
    let kernel_normalization = h2 * h * PI / 8.0;
    // let kernel_normalization = h2 * 7.0 * PI / 40.0; // 2-D normalization
    let k_rh = 1.0 - r / h;
    let k_hi = -k_rh * k_rh;
    let k_lo = 3.0 * (r2 / h2) - 2.0 * (r /h);
    var dkernel = 0.0;
    if (r < 0.5 * h) {
        dkernel = 6.0 * k_lo / kernel_normalization;
    } else if (r <= h) {
        dkernel = 6.0 * k_hi / kernel_normalization;
    }
    return dkernel;
}

// Spiky
fn kernel_spiky(r: f32, h: f32, h2: f32) -> f32 {
    let kernel_normalization = h2 * h2 * h2 * PI / 15.0; // 3-D normalization
    // let kernel_normalization = h2 * h2 * h * PI / 30.0; // 2-D normalization
    let k_rh = (h - r);
    var kernel = 0.0;
    if (r < h) {
        kernel = k_rh * k_rh * k_rh / kernel_normalization;
    }
    return kernel;
}
// Spiky Derivative
fn dkernel_spiky(r: f32, h: f32, h2: f32) -> f32 {
    let kernel_normalization = h2 * h2 * h2 * PI / 15.0; // 3-D normalization
    // let kernel_normalization = h2 * h2 * h * PI / 30.0; // 2-D normalization
    let k_rh = (h - r);
    var dkernel = 0.0;
    if (r < h) {
        dkernel = -3.0 * k_rh * k_rh / kernel_normalization;
    }
    return dkernel;
}


