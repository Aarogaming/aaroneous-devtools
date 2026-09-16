// Compute Enzyme - Performs Monte Carlo sampling and writes results to Synapse
// This enzyme demonstrates WASM-host compute delegation

use std::ffi::CString;
use std::os::raw::c_char;

// Host functions provided by the Hypervisor
#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn synapse_write(offset: u32, ptr: *const c_char, len: u32) -> i32;
    fn synapse_read(offset: u32, buffer: *mut c_char, len: u32) -> i32;
    fn get_timestamp() -> u64;
}

#[cfg(not(target_arch = "wasm32"))]
#[no_mangle]
pub extern "C" fn synapse_write(_offset: u32, _ptr: *const c_char, _len: u32) -> i32 {
    0
}

#[cfg(not(target_arch = "wasm32"))]
#[no_mangle]
pub extern "C" fn synapse_read(_offset: u32, _buffer: *mut c_char, _len: u32) -> i32 {
    0
}

#[cfg(not(target_arch = "wasm32"))]
#[no_mangle]
pub extern "C" fn get_timestamp() -> u64 {
    0
}

/// Simple pseudo-random number generator (Xorshift32)
struct Xorshift32 {
    state: u32,
}

impl Xorshift32 {
    fn new(seed: u32) -> Self {
        Self { state: seed | 1 } // Ensure non-zero
    }

    fn next(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Generate a float in [0, 1)
    fn next_f64(&mut self) -> f64 {
        (self.next() % 1000000) as f64 / 1000000.0
    }

    /// Box-Muller transform for normal distribution
    fn next_normal(&mut self, mean: f64, std_dev: f64) -> f64 {
        let u1 = self.next_f64().max(1e-10);
        let u2 = self.next_f64();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        mean + std_dev * z
    }
}

/// Run Monte Carlo simulation and write results to synapse
#[no_mangle]
pub extern "C" fn run() -> i32 {
    let timestamp = unsafe { get_timestamp() };
    let mut rng = Xorshift32::new(timestamp as u32);

    // Parameters for Monte Carlo
    let iterations = 1000;
    let mean = 0.5;
    let std_dev = 0.15;

    // Run simulation
    let mut sum = 0.0;
    let mut sum_sq = 0.0;
    let mut min_val = f64::MAX;
    let mut max_val = f64::MIN;
    let mut samples: Vec<f64> = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let sample = rng.next_normal(mean, std_dev);
        samples.push(sample);
        sum += sample;
        sum_sq += sample * sample;
        if sample < min_val {
            min_val = sample;
        }
        if sample > max_val {
            max_val = sample;
        }
    }

    // Calculate statistics
    let calc_mean = sum / iterations as f64;
    let variance = (sum_sq / iterations as f64) - (calc_mean * calc_mean);
    let calc_std = variance.sqrt();

    // Sort for percentiles
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p5 = samples[(iterations as f64 * 0.05) as usize];
    let p50 = samples[(iterations as f64 * 0.50) as usize];
    let p95 = samples[(iterations as f64 * 0.95) as usize];

    // Format result as JSON-like string
    let result = format!(
        "{{\"type\":\"monte_carlo\",\"mean\":{:.4},\"std\":{:.4},\"min\":{:.4},\"max\":{:.4},\"p5\":{:.4},\"p50\":{:.4},\"p95\":{:.4},\"iterations\":{}}}",
        calc_mean, calc_std, min_val, max_val, p5, p50, p95, iterations
    );

    // Write to synapse at offset 500 (compute results region)
    let msg = CString::new(result).unwrap();
    let ptr = msg.as_ptr();
    let len = msg.as_bytes().len() as u32;

    unsafe {
        let write_result = synapse_write(500, ptr, len);
        if write_result != 0 {
            return -1;
        }

        // Write status marker
        let status = CString::new("COMPUTE_DONE").unwrap();
        synapse_write(
            8,
            status.as_ptr() as *const c_char,
            status.as_bytes().len() as u32,
        );
    }

    1 // Success
}

/// Run entropy calculation on input data
#[no_mangle]
pub extern "C" fn run_entropy() -> i32 {
    // Read input from synapse offset 400
    let mut buffer = [0u8; 1024];
    let len = unsafe { synapse_read(400, buffer.as_mut_ptr() as *mut c_char, 1024) };

    if len <= 0 {
        return -1;
    }

    // Calculate Shannon entropy
    let mut freq = [0u32; 256];
    for &byte in &buffer[..len as usize] {
        freq[byte as usize] += 1;
    }

    let total = len as f64;
    let mut entropy = 0.0;
    for &count in &freq {
        if count > 0 {
            let p = count as f64 / total;
            entropy -= p * p.log2();
        }
    }

    // Write result
    let result = format!(
        "{{\"type\":\"entropy\",\"value\":{:.4},\"bytes\":{}}}",
        entropy, len
    );
    let msg = CString::new(result).unwrap();

    unsafe {
        synapse_write(500, msg.as_ptr(), msg.as_bytes().len() as u32);

        let status = CString::new("COMPUTE_DONE").unwrap();
        synapse_write(
            8,
            status.as_ptr() as *const c_char,
            status.as_bytes().len() as u32,
        );
    }

    1
}
