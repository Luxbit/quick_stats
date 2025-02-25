use tch::{Cuda, Device};

#[derive(Debug)]
pub struct GpuInfo {
    pub device_id: usize,
    pub device: Device,
    pub name: Option<String>,
    pub total_memory: Option<u64>,
    pub free_memory: Option<u64>,
    pub used_memory: Option<u64>,
}

pub fn get_gpu_info() -> Vec<GpuInfo> {
    let device_count = Cuda::device_count() as usize;
    let mut gpu_info_list = Vec::new();

    if device_count == 0 {
        return gpu_info_list;
    }

    for device_id in 0..device_count {
        let device = Device::Cuda(device_id);
        let gpu_info = GpuInfo {
            device_id,
            device,
            name: None,
            total_memory: None,
            free_memory: None,
            used_memory: None,
        };
        gpu_info_list.push(gpu_info);
    }

    gpu_info_list
}
