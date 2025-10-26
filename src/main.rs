mod benchmark;
mod helpers;
mod info;

use benchmark::{cpu::benchmark_cpu, gpu::benchmark_gpu};
use clap::{Arg, Command};
use info::cpu::get_cpu_info;
use info::gpu::get_gpu_info;
use info::network::{get_internet_speed, get_ping, get_public_ip};
use info::power::{get_battery_info, BatteryInfo};
use serde_json::{json, Value};
use std::fs::File;
use std::io::{self, Write};
use tch::Device;
use tokio::runtime::Runtime;

fn main() -> io::Result<()> {
    let matches = configure_cli();
    let output_format = matches.get_one::<String>("format").unwrap();
    let output_file = matches.get_one::<String>("outputFile");
    let features: Vec<&String> = matches.get_many::<String>("features").unwrap().collect();
    let no_benchmark = matches.get_flag("no-benchmark");
    let no_bandwidth = matches.get_flag("no-bandwidth");

    let mut cpu_info = None;
    let mut cpu_gflops = None;
    let mut cpu_elapsed_time = None;
    let mut battery_info = None;
    let mut gpu_results = None;
    let mut ping = None;
    let mut public_ip = None;
    let mut internet_speed = None;

    if features.contains(&&"cpu".to_string()) {
        let cpu_info_data = get_cpu_info();
        cpu_info = Some(cpu_info_data);

        if !no_benchmark {
            let (gflops, elapsed_time) = benchmark_cpu(5);
            cpu_gflops = Some(gflops);
            cpu_elapsed_time = Some(elapsed_time);
        }
    }

    if features.contains(&&"gpu".to_string()) {
        // Get CPU info if we don't have it yet (needed for MPS detection)
        if cpu_info.is_none() {
            cpu_info = Some(get_cpu_info());
        }

        let supports_mps = cpu_info.as_ref().map_or(false, |info| {
            info.arch == Some("arm64".to_string()) && info.os == "macos"
        });
        gpu_results = if supports_mps {
            Some(get_mps_gpu_info(no_benchmark)?)
        } else {
            Some(get_cuda_gpus_info(no_benchmark)?)
        };
    }

    if features.contains(&&"battery".to_string()) {
        battery_info = Some(get_battery_info());
    }

    if features.contains(&&"network".to_string()) {
        // Create a new Tokio runtime
        let rt = Runtime::new()?;
        // Use the runtime to block on the async functions
        ping = rt.block_on(get_ping()).ok();
        public_ip = rt.block_on(get_public_ip()).ok();

        // Only run speed test if ping was successful and not disabled
        if !no_bandwidth && ping.is_some() {
            internet_speed = rt.block_on(get_internet_speed()).ok();
        }
    }

    let output = match output_format.as_str() {
        "json" => generate_json_output(
            cpu_info.as_ref(),
            cpu_gflops,
            cpu_elapsed_time,
            battery_info.as_ref(),
            gpu_results.as_ref(),
            ping,
            public_ip.as_ref(),
            internet_speed.as_ref(),
        )?,
        _ => generate_plain_output(
            cpu_info.as_ref(),
            cpu_gflops,
            cpu_elapsed_time,
            battery_info.as_ref(),
            gpu_results.as_ref(),
            ping,
            public_ip.as_ref(),
            internet_speed.as_ref(),
        ),
    };

    write_output(output_file, &output)
}

fn configure_cli() -> clap::ArgMatches {
    Command::new("System Benchmark")
        .version("1.0")
        .about("Benchmarks CPU and GPU performance, and provides battery information")
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .value_name("FORMAT")
                .help("Sets the output format: plain or json")
                .default_value("plain"),
        )
        .arg(
            Arg::new("outputFile")
                .short('o')
                .long("outputFile")
                .value_name("OUTPUT")
                .help("Specifies the file to write the output to"),
        )
        .arg(
            Arg::new("features")
                .short('e')
                .long("features")
                .value_name("FEATURE")
                .help("Select which benchmarks/features to run/enable: cpu, gpu, battery, network (comma-separated)")
                .default_value("cpu,gpu,battery,network")
                .use_value_delimiter(true),
        )
        .arg(
            Arg::new("no-benchmark")
                .long("no-benchmark")
                .help("Skip CPU and GPU benchmarks, only collect device information")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-bandwidth")
                .long("no-bandwidth")
                .help("Skip internet speed measurement")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches()
}
fn generate_json_output(
    cpu_info: Option<&info::cpu::CpuInfo>,
    cpu_gflops: Option<f64>,
    cpu_elapsed_time: Option<f64>,
    battery_info: Option<&BatteryInfo>,
    gpu_results: Option<&Vec<Value>>,
    ping: Option<u32>,
    public_ip: Option<&String>,
    internet_speed: Option<&(f64, f64)>,
) -> Result<String, serde_json::Error> {
    let mut output_json = json!({});

    if let Some(info) = cpu_info {
        // General information
        output_json["general"] = json!({
            "os": info.os,
            "os_version": info.os_version.as_deref().unwrap_or("Not available"),
        });

        // Memory information
        output_json["memory"] = json!({
            "total_memory_mb": info.total_memory,
            "used_memory_mb": info.used_memory,
            "total_swap_mb": info.total_swap,
            "used_swap_mb": info.used_swap,
        });

        // CPU-specific information
        let mut cpu_json = json!({
            "arch": info.arch.as_deref().unwrap_or("Not available"),
            "cpu_count": info.cpu_count,
            "vendor_id": info.vendor_id.as_deref().unwrap_or("Not available"),
            "brand": info.brand.as_deref().unwrap_or("Not available"),
            "frequency_mhz": info.frequency.unwrap_or(0),
        });

        if let Some(gflops) = cpu_gflops {
            cpu_json["gflops"] = json!(gflops);
        }

        if let Some(elapsed) = cpu_elapsed_time {
            cpu_json["benchmark_duration_seconds"] = json!(elapsed);
        }

        output_json["cpu"] = cpu_json;
    }

    if let Some(gpu) = gpu_results {
        output_json["gpu"] = json!(gpu);
    }

    if let Some(battery) = battery_info {
        output_json["battery"] = json!({
            "has_battery": battery.has_battery,
            "charge_percent": battery.charge_percent,
            "is_charging": battery.is_charging,
            "on_ac_power": battery.on_ac_power,
            "wh_capacity": battery.wh_capacity,
        });
    }

    // Group network-related information
    let mut network = json!({});

    // Add online status based on ping success
    network["online"] = json!(ping.is_some());

    if let Some(ping_value) = ping {
        network["ping_ms"] = json!(ping_value);
    }

    if let Some(ip) = public_ip {
        network["public_ip"] = json!(ip);
    }

    if let Some((download, upload)) = internet_speed {
        network["speed"] = json!({
            "download_mbps": download,
            "upload_mbps": upload
        });
    }

    // Only add the network if it's not empty
    if !network.as_object().unwrap().is_empty() {
        output_json["network"] = network;
    }

    serde_json::to_string_pretty(&output_json)
}

fn generate_plain_output(
    cpu_info: Option<&info::cpu::CpuInfo>,
    cpu_gflops: Option<f64>,
    cpu_elapsed_time: Option<f64>,
    battery_info: Option<&BatteryInfo>,
    gpu_results: Option<&Vec<serde_json::Value>>,
    ping: Option<u32>,
    public_ip: Option<&String>,
    internet_speed: Option<&(f64, f64)>,
) -> String {
    let mut output = String::new();

    if let Some(info) = cpu_info {
        output.push_str(&format_general_info(info));
        output.push_str(&format_cpu_info(
            info,
            cpu_gflops.unwrap_or(0.0),
            cpu_elapsed_time.unwrap_or(0.0),
        ));
        output.push_str(&format_memory_info(info));
    }

    if let Some(gpu) = gpu_results {
        if let Some(supports_mps) = cpu_info
            .as_ref()
            .map(|info| info.arch == Some("arm64".to_string()) && info.os == "macos")
        {
            if supports_mps {
                output.push_str(&format_mps_gpu_plain(gpu));
            } else {
                output.push_str(&format_cuda_gpus_plain(gpu));
            }
        }
    }

    if let Some(battery) = battery_info {
        output.push_str(&format_battery_info(battery));
    }

    // Add network section if any network feature was checked
    if ping.is_some() || public_ip.is_some() || internet_speed.is_some() {
        output.push_str("=> Network:\n");
        output.push_str(&format!("Online: {}\n", ping.is_some()));

        if let Some(p) = ping {
            output.push_str(&format!("Internet Ping: {:.2} ms\n", p));
        }

        if let Some(ip) = public_ip {
            output.push_str(&format!("Public IP: {}\n", ip));
        }

        if let Some((download, upload)) = internet_speed {
            output.push_str(&format!("Download speed: {:.2} Mbps (minimum)\n", download));
            output.push_str(&format!("Upload speed: {:.2} Mbps (minimum)\n", upload));
        }
    }
    output
}

fn format_general_info(cpu_info: &info::cpu::CpuInfo) -> String {
    format!(
        "=> General:\n\
        OS          : {}\n\
        OS version  : {}\n\n",
        cpu_info.os,
        cpu_info.os_version.as_deref().unwrap_or("Not available")
    )
}

fn format_memory_info(cpu_info: &info::cpu::CpuInfo) -> String {
    format!(
        "=> Memory:\n\
        Total       : {} mb\n\
        Used        : {} mb\n\
        Swap Total  : {} mb\n\
        Swap Used   : {} mb\n\n",
        cpu_info.total_memory, cpu_info.used_memory, cpu_info.total_swap, cpu_info.used_swap
    )
}

fn format_cpu_info(
    cpu_info: &info::cpu::CpuInfo,
    cpu_gflops: f64,
    cpu_elapsed_time: f64,
) -> String {
    let mut output = format!(
        "=> CPU:\n\
        Vendor      : {}\n\
        Brand       : {}\n\
        Architecture: {}\n\
        Frequency   : {} MHz\n\
        Count       : {}\n",
        cpu_info.vendor_id.as_deref().unwrap_or("Not available"),
        cpu_info.brand.as_deref().unwrap_or("Not available"),
        cpu_info.arch.as_deref().unwrap_or("Not available"),
        cpu_info.frequency.unwrap_or(0),
        cpu_info.cpu_count,
    );

    if cpu_gflops > 0.0 {
        output.push_str(&format!("        FLOPS       : {:.2} GFLOPS\n", cpu_gflops));
    }

    if cpu_elapsed_time > 0.0 {
        output.push_str(&format!(
            "        Benchmark duration: {:.2} seconds\n",
            cpu_elapsed_time
        ));
    }

    output.push('\n');
    output
}

fn format_battery_info(battery_info: &BatteryInfo) -> String {
    let charge = if battery_info.charge_percent.is_some() {
        format!("{}%", battery_info.charge_percent.unwrap().to_string())
    } else {
        "None".to_string()
    };
    let capacity = if battery_info.wh_capacity.is_some() {
        format!("{} Wh", battery_info.wh_capacity.unwrap().to_string())
    } else {
        "None".to_string()
    };

    format!(
        "=> Power:\n\
        Battery         : {:?}\n\
        State of charge : {}\n\
        Charging        : {:?}\n\
        On AC power     : {:?}\n\
        Capacity        : {}\n\n",
        battery_info.has_battery,
        charge,
        battery_info.is_charging.unwrap_or(false),
        battery_info.on_ac_power.unwrap_or(false),
        capacity
    )
}

fn get_mps_gpu_info(no_benchmark: bool) -> io::Result<Vec<serde_json::Value>> {
    let gpu_infos = get_gpu_info();
    let mut gpu_results = Vec::new();

    for info in gpu_infos.into_iter() {
        // Only process MPS devices
        if matches!(info.device, Device::Mps) {
            let mut gpu_json = json!({
                "device": "MPS",
                "name": info.name.unwrap_or_else(|| "Not available".to_string()),
                "total_memory_mb": info.total_memory.unwrap_or(0),
                "used_memory_mb": info.used_memory.unwrap_or(0),
                "metal_support": info.metal_support.unwrap_or_else(|| "Not available".to_string()),
                "core_count": info.core_count.unwrap_or(0),
                "vendor": info.vendor.unwrap_or_else(|| "Not available".to_string()),
            });

            if !no_benchmark {
                let (gpu_tflops, gpu_elapsed_time) = benchmark_gpu(Device::Mps, 1000);
                gpu_json["tflops"] = json!(gpu_tflops);
                gpu_json["duration"] = json!(gpu_elapsed_time);
            }

            gpu_results.push(gpu_json);
        }
    }

    Ok(gpu_results)
}

fn get_cuda_gpus_info(no_benchmark: bool) -> io::Result<Vec<serde_json::Value>> {
    let gpu_infos = get_gpu_info();
    let mut gpu_results = Vec::new();

    for (index, info) in gpu_infos.into_iter().enumerate() {
        let mut gpu_json = json!({
            "device_id": info.device_id,
            "device": format!("{:?}", info.device),
            "name": info.name.unwrap_or_else(|| "Not available".to_string()),
            "total_memory_mb": info.total_memory.unwrap_or(0),
            "used_memory_mb": info.used_memory.unwrap_or(0),
            "compute_capability": info.compute_capability.unwrap_or_else(|| "Not available".to_string()),
            "cuda_version": info.cuda_version.unwrap_or_else(|| "Not available".to_string()),
            "driver_version": info.driver_version.unwrap_or_else(|| "Not available".to_string()),
        });

        if !no_benchmark {
            let (gpu_tflops, gpu_elapsed_time) = benchmark_gpu(Device::Cuda(index), 1000);
            gpu_json["tflops"] = json!(gpu_tflops);
            gpu_json["duration"] = json!(gpu_elapsed_time);
        }

        gpu_results.push(gpu_json);
    }

    Ok(gpu_results)
}

fn format_mps_gpu_plain(gpu_data: &Vec<serde_json::Value>) -> String {
    let mut output = String::from("=> GPU:\n");

    if let Some(gpu) = gpu_data.first() {
        let name = gpu
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Not available");
        let vendor = gpu
            .get("vendor")
            .and_then(|v| v.as_str())
            .unwrap_or("Not available");
        let metal_support = gpu
            .get("metal_support")
            .and_then(|v| v.as_str())
            .unwrap_or("Not available");
        let core_count = gpu.get("core_count").and_then(|v| v.as_u64()).unwrap_or(0);
        let total_memory = gpu
            .get("total_memory_mb")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let used_memory = gpu
            .get("used_memory_mb")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        output.push_str(&format!(
            "        Name                : {}\n\
             Vendor              : {}\n\
             Metal Support       : {}\n\
             Core Count          : {}\n\
             Unified Memory Total: {} MB\n\
             Unified Memory Used : {} MB\n",
            name, vendor, metal_support, core_count, total_memory, used_memory
        ));

        if let Some(tflops) = gpu.get("tflops") {
            output.push_str(&format!(
                "        GPU FLOPS           : {:.2} TFLOPS\n",
                tflops.as_f64().unwrap_or(0.0)
            ));
        }
        if let Some(duration) = gpu.get("duration") {
            output.push_str(&format!(
                "        Benchmark Duration  : {:.2} seconds\n",
                duration.as_f64().unwrap_or(0.0)
            ));
        }
    }

    output.push('\n');
    output
}

fn format_cuda_gpus_plain(gpu_data: &Vec<serde_json::Value>) -> String {
    let mut output = String::from("=> GPU:\n");

    for gpu in gpu_data {
        let device_id = gpu.get("device_id").and_then(|v| v.as_u64()).unwrap_or(0);
        let device = gpu
            .get("device")
            .and_then(|v| v.as_str())
            .unwrap_or("Not available");
        let name = gpu
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Not available");
        let total_memory = gpu
            .get("total_memory_mb")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let used_memory = gpu
            .get("used_memory_mb")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let compute_capability = gpu
            .get("compute_capability")
            .and_then(|v| v.as_str())
            .unwrap_or("Not available");
        let cuda_version = gpu
            .get("cuda_version")
            .and_then(|v| v.as_str())
            .unwrap_or("Not available");
        let driver_version = gpu
            .get("driver_version")
            .and_then(|v| v.as_str())
            .unwrap_or("Not available");

        output.push_str(&format!(
            "CUDA Device {} Information:\n\
            Device: {}\n\
            Name: {}\n\
            Total Memory: {} MB\n\
            Used Memory: {} MB\n\
            Compute Capability: {}\n\
            CUDA Version: {}\n\
            Driver Version: {}\n",
            device_id,
            device,
            name,
            total_memory,
            used_memory,
            compute_capability,
            cuda_version,
            driver_version
        ));

        if let Some(tflops) = gpu.get("tflops") {
            output.push_str(&format!(
                "GPU Estimated FLOPS: {:.2} TFLOPS\n",
                tflops.as_f64().unwrap_or(0.0)
            ));
        }

        if let Some(duration) = gpu.get("duration") {
            output.push_str(&format!(
                "GPU benchmark duration: {:.2} seconds\n",
                duration.as_f64().unwrap_or(0.0)
            ));
        }
    }

    output
}
fn write_output(output_file: Option<&String>, output: &str) -> io::Result<()> {
    if let Some(file_path) = output_file {
        let mut file = File::create(file_path)?;
        file.write_all(output.as_bytes())?;
    } else {
        println!("{}", output);
    }
    Ok(())
}
