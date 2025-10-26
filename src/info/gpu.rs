use crate::helpers::bytes_to_megabytes;
use std::process::Command;
use sysinfo::System;
use tch::{Cuda, Device};

#[derive(Debug)]
pub struct GpuInfo {
    pub device_id: usize,
    pub device: Device,
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

pub fn get_gpu_info() -> Vec<GpuInfo> {
    let mut gpu_info_list = Vec::new();

    // Check for CUDA devices
    let cuda_device_count = Cuda::device_count() as usize;
    for device_id in 0..cuda_device_count {
        let device = Device::Cuda(device_id);
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
            device,
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
                device: Device::Mps,
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
