use std::time::Instant;
use ndarray::linalg::general_mat_mul;
use ndarray::Array2;

/// Iterations used by `main`. Reported in the JSON output so the FLOPS figure
/// can be reproduced from `benchmark_duration_seconds` alone.
pub const ITERATIONS: u64 = 5;

/// Matrix edge length: an `SIZE x SIZE` GEMM produces `2 * SIZE^3` FLOPs.
pub const SIZE: usize = 1024;

/// Precision actually measured below (`Array2<f64>`). Surfaced in the JSON
/// output as `"precision"`; keep in sync with the element type of the
/// operands, and with `gpu::PRECISION` differences being intentional.
pub const PRECISION: &str = "fp64";

pub fn benchmark_cpu(iterations: u64) -> (f64, f64) {
    let size = SIZE; // Size of the matrix (nxn) -> produces 2n^3 ops

    // Record the start time
    let start = Instant::now();

    // Pre-allocate matrices (FP64 / double precision: see `PRECISION`)
    let mat_a = Array2::<f64>::zeros((size, size));
    let mat_b = Array2::<f64>::zeros((size, size));
    let mut mat_c = Array2::<f64>::zeros((size, size));

    // Perform the matrix multiplication multiple times
    for _ in 0..iterations {
        general_mat_mul(1.0, &mat_a, &mat_b, 0.0, &mut mat_c);
    }

    // Record the end time
    let duration = start.elapsed().as_secs_f64();

    // Calculate the number of floating-point operations
    let flops_per_mul = 2.0 * (size as f64).powi(3);
    let total_flops = flops_per_mul * (iterations as f64);

    // Calculate TFLOPS
    let gflops = total_flops / (duration * 1e9);

    (gflops, duration)
}

