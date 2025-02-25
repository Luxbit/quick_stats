use sysinfo::System;
use crate::helpers::bytes_to_megabytes;

#[derive(Debug)]
pub struct CpuInfo {
    pub total_memory: u64,
    pub used_memory: u64,
    pub total_swap: u64,
    pub used_swap: u64,
    pub arch: Option<String>,
    pub os: String,
    pub os_version: Option<String>,
    pub cpu_count: usize,
}

pub fn get_cpu_info() -> CpuInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    CpuInfo {
        total_memory: bytes_to_megabytes(sys.total_memory()),
        used_memory: bytes_to_megabytes(sys.used_memory()),
        total_swap: bytes_to_megabytes(sys.total_swap()),
        used_swap: bytes_to_megabytes(sys.used_swap()),
        os: System::distribution_id(),
        arch: System::cpu_arch(),
        os_version: System::os_version(),
        cpu_count: sys.cpus().len(),
    }
}
