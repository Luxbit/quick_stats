use crate::info::gpu::Accelerator;

/// Iterations used by `main`. Reported in the JSON output so the FLOPS figure
/// can be reproduced from `duration` alone.
pub const ITERATIONS: u64 = 1000;

/// Matrix edge length: an `SIZE x SIZE` GEMM produces `2 * SIZE^3` FLOPs.
pub const SIZE: usize = 1024;

/// Precision actually measured below (f32 in both backends). Surfaced in the
/// JSON output as `"precision"`. Note this differs from `cpu::PRECISION`
/// (`fp64`), so the two FLOPS figures are not directly comparable.
pub const PRECISION: &str = "fp32";

/// Benchmark the given accelerator with a 1024x1024 float32 GEMM and return
/// `(tflops, duration_seconds, execution_provider)`. `tflops`/`duration` are
/// `None` when the benchmark could not run (missing onnxruntime, no EP…); the
/// JSON output then omits them and reports `ep = "unavailable"`.
pub fn benchmark_gpu(device: Accelerator, iterations: u64) -> (Option<f64>, Option<f64>, &'static str) {
    #[cfg(target_os = "macos")]
    return macos::benchmark_gpu(device, iterations);
    #[cfg(not(target_os = "macos"))]
    return onnx::benchmark_gpu(device, iterations);
}

// ---------------------------------------------------------------------------
// macOS: libtorch / MPS (unchanged behaviour, same matmul as 1.x releases)
// ---------------------------------------------------------------------------
#[cfg(target_os = "macos")]
mod macos {
    use super::Accelerator;
    use std::time::Instant;
    use tch::{Device, Tensor};

    pub fn benchmark_gpu(device: Accelerator, iterations: u64) -> (Option<f64>, Option<f64>, &'static str) {
        let (tch_device, ep) = match device {
            Accelerator::Cuda(i) => (Device::Cuda(i), "CUDA"),
            Accelerator::Mps => (Device::Mps, "MPS"),
            Accelerator::Cpu => (Device::Cpu, "CPU"),
        };
        let size = super::SIZE as i64;

        // Create two large tensors filled with random values (FP32: see
        // `super::PRECISION`)
        let a = Tensor::rand(&[size, size], (tch::Kind::Float, tch_device));
        let b = Tensor::rand(&[size, size], (tch::Kind::Float, tch_device));

        // Warm up the GPU by performing some operations
        for _ in 0..10 {
            let _ = a.matmul(&b);
        }

        // Record the start time
        let start = Instant::now();

        // Perform the matrix multiplication multiple times
        for _ in 0..iterations {
            let _ = a.matmul(&b);
        }

        // Record the end time
        let duration = start.elapsed().as_secs_f64();

        // Calculate the number of floating-point operations
        let flops_per_mul = 2.0 * (size as f64).powi(3);
        let total_flops = flops_per_mul * (iterations as f64);

        // Calculate TFLOPS
        let tflops = total_flops / (duration * 1e12);

        (Some(tflops), Some(duration), ep)
    }
}

// ---------------------------------------------------------------------------
// Windows/Linux: ONNX Runtime
//
// The committed weightless GEMM model (utils/gen_gemm_onnx.py) has both
// operand matrices as graph inputs, so the .onnx is ~160 bytes and the FLOPS
// math matches the libtorch benchmark exactly (2 * 1024^3 per invocation).
// ---------------------------------------------------------------------------
#[cfg(not(target_os = "macos"))]
mod onnx {
    use super::Accelerator;
    use ort::session::Session;
    use ort::value::Tensor;
    use std::path::PathBuf;
    use std::time::Instant;

    static GEMM_MODEL: &[u8] = include_bytes!("../../models/gemm_1024.onnx");
    /// Must match the committed ONNX model's geometry (1024x1024 FLOAT = f32).
    const SIZE: i64 = super::SIZE as i64;

    /// Locate the onnxruntime shared library ort will dlopen, without letting
    /// ort itself try first: a failed dylib load PANICS inside ort (it cannot
    /// be caught), so we pre-flight everything and return `None` (reported as
    /// "unavailable") when nothing resolvable exists.
    ///
    /// Resolution order (mirrors doc_processor's ORT_DYLIB_PATH contract):
    ///   1. `ORT_DYLIB_PATH` — must exist, else unavailable (never passes a
    ///      broken path down to ort's panic)
    ///   2. `<exe_dir>/onnxruntime.dll` / `libonnxruntime.so`
    fn resolve_dylib() -> Option<()> {
        if let Some(p) = std::env::var_os("ORT_DYLIB_PATH") {
            let p = PathBuf::from(&p);
            if p.as_os_str().is_empty() {
                return None;
            }
            if !p.exists() {
                eprintln!(
                    "quick_stats: ORT_DYLIB_PATH points at a missing file ({}); GPU benchmark unavailable",
                    p.display()
                );
                return None;
            }
            return Some(());
        }
        let Ok(exe) = std::env::current_exe() else { return None };
        let Some(dir) = exe.parent() else { return None };
        #[cfg(target_os = "windows")]
        let name = "onnxruntime.dll";
        #[cfg(not(target_os = "windows"))]
        let name = "libonnxruntime.so";
        let candidate = dir.join(name);
        if candidate.exists() {
            std::env::set_var("ORT_DYLIB_PATH", &candidate);
            return Some(());
        }
        None
    }

    fn build_session(device: Accelerator) -> Result<(Session, &'static str), String> {
        // SessionBuilder methods take `self` by value, so each attempt owns
        // its builder; a failed EP registration (error_on_failure) falls
        // through to the next rung of the ladder.
        match device {
            Accelerator::Cuda(idx) => {
                let attempt = (|| -> Result<Session, String> {
                    let cuda = ort::ep::CUDA::default().with_device_id(idx as i32).build().error_on_failure();
                    let b = Session::builder().map_err(|e| format!("builder: {e}"))?;
                    let mut b = b.with_execution_providers([cuda]).map_err(|e| format!("cuda ep: {e}"))?;
                    b.commit_from_memory(GEMM_MODEL).map_err(|e| format!("commit: {e}"))
                })();
                if let Ok(s) = attempt {
                    return Ok((s, "CUDA"));
                }
            }
            Accelerator::Mps | Accelerator::Cpu => {}
        }

        // DirectML: any DX12 GPU (NVIDIA/AMD/Intel) on Windows without a CUDA
        // install. Only attempted when CUDA was not already selected.
        #[cfg(target_os = "windows")]
        if !matches!(device, Accelerator::Cpu) {
            let attempt = (|| -> Result<Session, String> {
                let dml = ort::ep::DirectML::default().build().error_on_failure();
                let b = Session::builder().map_err(|e| format!("builder: {e}"))?;
                let mut b = b.with_execution_providers([dml]).map_err(|e| format!("directml ep: {e}"))?;
                b.commit_from_memory(GEMM_MODEL).map_err(|e| format!("commit: {e}"))
            })();
            if let Ok(s) = attempt {
                return Ok((s, "DirectML"));
            }
        }

        // CPU execution provider — always present in every onnxruntime build.
        let mut b = Session::builder().map_err(|e| format!("builder: {e}"))?;
        let session = b.commit_from_memory(GEMM_MODEL).map_err(|e| format!("commit: {e}"))?;
        Ok((session, "CPU"))
    }

    fn deterministic_matrix(seed: u32) -> Vec<f32> {
        // xorshift32 — deterministic across runs and machines, no deps
        let n = (SIZE * SIZE) as usize;
        let mut v = Vec::with_capacity(n);
        let mut state = seed;
        for _ in 0..n {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            v.push((state % 2000) as f32 / 1000.0 - 1.0);
        }
        v
    }

    pub fn benchmark_gpu(device: Accelerator, iterations: u64) -> (Option<f64>, Option<f64>, &'static str) {
        if resolve_dylib().is_none() {
            eprintln!(
                "quick_stats: onnxruntime not found (set ORT_DYLIB_PATH or place the dylib next to the binary); GPU benchmark unavailable"
            );
            return (None, None, "unavailable");
        }

        let (mut session, ep) = match build_session(device) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("quick_stats: GPU benchmark unavailable ({e})");
                return (None, None, "unavailable");
            }
        };

        let a = deterministic_matrix(0x9E37_79B9);
        let b = deterministic_matrix(0x85EB_CA6B);
        let (ta, tb) = match (
            Tensor::from_array((vec![SIZE, SIZE], a)),
            Tensor::from_array((vec![SIZE, SIZE], b)),
        ) {
            (Ok(ta), Ok(tb)) => (ta, tb),
            (Err(e), _) | (_, Err(e)) => {
                eprintln!("quick_stats: GPU benchmark unavailable (input tensors: {e})");
                return (None, None, "unavailable");
            }
        };

        // Warm up (kernel/JIT compilation, EP init, caches)
        for _ in 0..10 {
            let _ = session.run(ort::inputs!["a" => &ta, "b" => &tb]);
        }

        let start = Instant::now();
        for _ in 0..iterations {
            let _ = session.run(ort::inputs!["a" => &ta, "b" => &tb]);
        }
        let duration = start.elapsed().as_secs_f64();

        // Same FLOPS math as the libtorch benchmark: 2 * size^3 per MatMul
        let flops_per_mul = 2.0 * (SIZE as f64).powi(3);
        let total_flops = flops_per_mul * (iterations as f64);
        let tflops = total_flops / (duration * 1e12);

        (Some(tflops), Some(duration), ep)
    }
}
