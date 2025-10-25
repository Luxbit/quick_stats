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
    pub free_memory: Option<u64>,
    pub used_memory: Option<u64>,
    // CUDA-specific fields
    pub cuda_version: Option<String>,
    pub driver_version: Option<String>,
    // MPS-specific fields
    pub metal_support: Option<String>,
    pub core_count: Option<u32>,
    pub vendor: Option<String>,
}

fn query_nvidia_gpu(
    device_id: usize,
) -> (
    Option<String>,
    Option<u64>,
    Option<u64>,
    Option<u64>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<u32>,
    Option<String>,
) {
    let output = Command::new("nvidia-smi")
        .args(&[
            "--query-gpu=name,memory.total,memory.free,memory.used,cuda_version,driver_version",
            "--format=csv,noheader,nounits",
            &format!("--id={}", device_id),
        ])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            if let Ok(result) = String::from_utf8(output.stdout) {
                let parts: Vec<&str> = result.trim().split(',').collect();
                if parts.len() == 6 {
                    let name = Some(parts[0].trim().to_string());
                    let total = parts[1].trim().parse::<u64>().ok();
                    let free = parts[2].trim().parse::<u64>().ok();
                    let used = parts[3].trim().parse::<u64>().ok();
                    let cuda_version = Some(parts[4].trim().to_string());
                    let driver_version = Some(parts[5].trim().to_string());
                    // CUDA: return name, total, free, used, cuda_version, driver_version, None for MPS fields
                    return (name, total, free, used, cuda_version, driver_version, None, None, None);
                }
            }
        }
    }

    (None, None, None, None, None, None, None, None, None)
}

fn query_mps_gpu() -> (
    Option<String>,
    Option<u64>,
    Option<u64>,
    Option<u64>,
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
        None, // free_memory - not used for MPS
        used_memory,
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
            free_memory,
            used_memory,
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
            free_memory,
            used_memory,
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
            free_memory,
            used_memory,
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
                free_memory,
                used_memory,
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
