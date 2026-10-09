use crate::helpers::bytes_to_megabytes;
use std::process::Command;
use sysinfo::System;

/// Backend-neutral accelerator handle, replacing the `tch::Device` that used
/// to thread through this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accelerator {
    Cuda(usize),
    Mps,
    /// Only used by the `--gpu-benchmark-smoke` CI flag — never reported as a
    /// discovered device.
    Cpu,
}

#[derive(Debug)]
pub struct GpuInfo {
    pub device_id: usize,
    pub device: Accelerator,
    pub name: Option<String>,
    pub total_memory: Option<u64>,
    pub used_memory: Option<u64>,
    // CUDA-specific fields
    pub compute_capability: Option<String>,
    pub cuda_version: Option<String>,
    pub driver_version: Option<String>,
    // MPS-specific fields
    pub metal_support: Option<String>,
    pub core_count: Option<u32>,
    pub vendor: Option<String>,
}

/// (Windows/Linux) nvidia-smi parsers — the whole CUDA info path.
#[cfg(not(target_os = "macos"))]
fn get_nvidia_versions() -> (Option<String>, Option<String>) {
    let output = Command::new("nvidia-smi")
        .args(&["--version"])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            if let Ok(result) = String::from_utf8(output.stdout) {
                let mut cuda_version = None;
                let mut driver_version = None;

                for line in result.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("CUDA Version") {
                        cuda_version = trimmed
                            .split(':')
                            .nth(1)
                            .map(|s| s.trim().to_string());
                    } else if trimmed.starts_with("DRIVER version") {
                        driver_version = trimmed
                            .split(':')
                            .nth(1)
                            .map(|s| s.trim().to_string());
                    }
                }

                return (cuda_version, driver_version);
            }
        }
    }

    (None, None)
}

/// (Windows/Linux) nvidia-smi parsers — the whole CUDA info path.
#[cfg(not(target_os = "macos"))]
fn get_nvidia_gpu_name(device_id: usize) -> Option<String> {
    let output = Command::new("nvidia-smi")
        .args(&["-L"])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            if let Ok(result) = String::from_utf8(output.stdout) {
                // Parse output like: "GPU 0: NVIDIA GeForce RTX 3090 (UUID: GPU-41f4e6bf-cee3-8098-d932-3a932203fa0c)"
                for line in result.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with(&format!("GPU {}:", device_id)) {
                        // Extract name between "GPU X: " and " ("
                        let prefix = format!("GPU {}: ", device_id);
                        if let Some(after_prefix) = trimmed.strip_prefix(&prefix) {
                            // Split by '(' to remove everything in parentheses
                            if let Some(name_part) = after_prefix.split('(').next() {
                                return Some(name_part.trim().to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// (Windows/Linux) nvidia-smi parsers — the whole CUDA info path.
#[cfg(not(target_os = "macos"))]
fn query_nvidia_gpu(
    device_id: usize,
) -> (
    Option<String>,
    Option<u64>,
    Option<u64>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<u32>,
    Option<String>,
) {
    // Get GPU name
    let name = get_nvidia_gpu_name(device_id);

    // Get GPU memory and compute capability
    let output = Command::new("nvidia-smi")
        .args(&[
            "--query-gpu=memory.total,memory.used,compute_cap",
            "--format=csv,noheader,nounits",
            &format!("--id={}", device_id),
        ])
        .output();

    let mut total = None;
    let mut used = None;
    let mut compute_capability = None;

    if let Ok(output) = output {
        if output.status.success() {
            if let Ok(result) = String::from_utf8(output.stdout) {
                let parts: Vec<&str> = result.trim().split(',').collect();
                if parts.len() == 3 {
                    total = parts[0].trim().parse::<u64>().ok();
                    used = parts[1].trim().parse::<u64>().ok();
                    compute_capability = Some(parts[2].trim().to_string());
                }
            }
        }
    }

    // Get CUDA and driver versions (same for all GPUs)
    let (cuda_version, driver_version) = get_nvidia_versions();

    (name, total, used, compute_capability, cuda_version, driver_version, None, None, None)
}

#[cfg(target_os = "macos")]
fn query_mps_gpu() -> (
    Option<String>,
    Option<u64>,
    Option<u64>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<u32>,
    Option<String>,
) {
    // Get GPU info from system_profiler
    let output = Command::new("system_profiler")
        .args(&["SPDisplaysDataType"])
        .output();

    let mut name = None;
    let mut metal_support = None;
    let mut core_count = None;
    let mut vendor = None;

    if let Ok(output) = output {
        if output.status.success() {
            if let Ok(result) = String::from_utf8(output.stdout) {
                // Parse chipset model, metal support, core count, and vendor
                for line in result.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("Chipset Model:") {
                        name = trimmed.split(':').nth(1).map(|s| s.trim().to_string());
                    } else if trimmed.starts_with("Metal Support:") {
                        metal_support = trimmed.split(':').nth(1).map(|s| s.trim().to_string());
                    } else if trimmed.starts_with("Total Number of Cores:") {
                        core_count = trimmed
                            .split(':')
                            .nth(1)
                            .and_then(|s| s.trim().parse::<u32>().ok());
                    } else if trimmed.starts_with("Vendor:") {
                        vendor = trimmed.split(':').nth(1).map(|s| s.trim().to_string());
                    }
                }
            }
        }
    }

    // Get unified memory info using sysinfo
    let mut sys = System::new_all();
    sys.refresh_memory();

    let total_memory = Some(bytes_to_megabytes(sys.total_memory()));
    let used_memory = Some(bytes_to_megabytes(sys.used_memory()));

    (
        name,
        total_memory,
        used_memory,
        None, // compute_capability - not applicable for MPS
        None, // cuda_version - not applicable for MPS
        None, // driver_version - not applicable for MPS
        metal_support,
        core_count,
        vendor,
    )
}

/// CUDA device discovery without libtorch: parse `nvidia-smi -L` (the same
/// source the per-GPU queries already use). Replaces `tch::Cuda::device_count`
/// which was always 0 in the released win/linux builds (CPU-only libtorch).
#[cfg(not(target_os = "macos"))]
fn cuda_device_ids() -> Vec<usize> {
    let output = match Command::new("nvidia-smi").arg("-L").output() {
        Ok(out) if out.status.success() => out,
        _ => return Vec::new(),
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            // "GPU 0: NVIDIA GeForce RTX 3090 (UUID: GPU-…)"
            let rest = line.trim().strip_prefix("GPU ")?;
            rest.split(':').next()?.trim().parse::<usize>().ok()
        })
        .collect()
}

pub fn get_gpu_info() -> Vec<GpuInfo> {
    let mut gpu_info_list = Vec::new();

    // CUDA devices (Windows/Linux only — libtorch always reported 0 here:
    // the CI builds linked CPU-only libtorch, see the plan doc)
    #[cfg(not(target_os = "macos"))]
    for device_id in cuda_device_ids() {
        let (
            name,
            total_memory,
            used_memory,
            compute_capability,
            cuda_version,
            driver_version,
            metal_support,
            core_count,
            vendor,
        ) = query_nvidia_gpu(device_id);

        let gpu_info = GpuInfo {
            device_id,
            device: Accelerator::Cuda(device_id),
            name,
            total_memory,
            used_memory,
            compute_capability,
            cuda_version,
            driver_version,
            metal_support,
            core_count,
            vendor,
        };
        gpu_info_list.push(gpu_info);
    }

    // Check for MPS device (Apple Silicon)
    // Only check on macOS systems
    #[cfg(target_os = "macos")]
    {
        let (
            name,
            total_memory,
            used_memory,
            compute_capability,
            cuda_version,
            driver_version,
            metal_support,
            core_count,
            vendor,
        ) = query_mps_gpu();

        // Only add MPS device if we successfully got the GPU name
        if name.is_some() {
            let gpu_info = GpuInfo {
                device_id: 0,
                device: Accelerator::Mps,
                name,
                total_memory,
                used_memory,
                compute_capability,
                cuda_version,
                driver_version,
                metal_support,
                core_count,
                vendor,
            };
            gpu_info_list.push(gpu_info);
        }
    }

    gpu_info_list
}
