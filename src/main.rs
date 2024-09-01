mod benchmark;
mod helpers;
mod info;

use benchmark::{cpu::benchmark_cpu, gpu::benchmark_gpu};
use clap::{Arg, Command};
use info::cpu::get_cpu_info;
use info::gpu::get_gpu_info;
use info::power::{get_battery_info, BatteryInfo};
use serde_json::json;
use std::fs::File;
use std::io::{self, Write};
use tch::Device;

fn main() -> io::Result<()> {
    let matches = configure_cli();
    let output_format = matches.get_one::<String>("output").unwrap();
    let output_file = matches.get_one::<String>("outputFile");

    let cpu_info = get_cpu_info();
    let (cpu_gflops, cpu_elapsed_time) = benchmark_cpu(5);
    let battery_info = get_battery_info();

    let output = match output_format.as_str() {
        "json" => generate_json_output(&cpu_info, cpu_gflops, cpu_elapsed_time, &battery_info)?,
        _ => generate_plain_output(&cpu_info, cpu_gflops, cpu_elapsed_time, &battery_info),
    };

    write_output(output_file, &output)
}

fn configure_cli() -> clap::ArgMatches {
    Command::new("System Benchmark")
        .version("1.0")
        .about("Benchmarks CPU and GPU performance")
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FORMAT")
                .help("Sets the output format: plain or json")
                .default_value("plain"),
        )
        .arg(
            Arg::new("outputFile")
                .short('f')
                .long("outputFile")
                .value_name("FILE")
                .help("Specifies the file to write the output to"),
        )
        .get_matches()
}

fn generate_json_output(
    cpu_info: &info::cpu::CpuInfo,
    cpu_gflops: f64,
    cpu_elapsed_time: f64,
    battery_info: &BatteryInfo,
) -> io::Result<String> {
    let supports_mps = cpu_info.arch == Some("arm64".to_string()) && cpu_info.os == "macos";
    let gpu_results = if supports_mps {
        benchmark_mps_gpu()?
    } else {
        benchmark_cuda_gpus()?
    };

    serde_json::to_string_pretty(&json!({
        "cpu_info": {
            "os": cpu_info.os,
            "os_version": cpu_info.os_version.as_deref().unwrap_or("Not available"),
            "total_memory_mb": cpu_info.total_memory,
            "used_memory_mb": cpu_info.used_memory,
            "total_swap_mb": cpu_info.total_swap,
            "used_swap_mb": cpu_info.used_swap,
            "arch": cpu_info.arch.as_deref().unwrap_or("Not available"),
            "cpu_count": cpu_info.cpu_count,
            "gflops": cpu_gflops,
            "benchmark_duration_seconds": cpu_elapsed_time
        },
        "gpu_info": gpu_results,
         "battery_info": {
            "has_battery": battery_info.has_battery,
            "charge_percent": battery_info.charge_percent,
        }
    }))
    .map_err(Into::into)
}

fn generate_plain_output(
    cpu_info: &info::cpu::CpuInfo,
    cpu_gflops: f64,
    cpu_elapsed_time: f64,
    battery_info: &BatteryInfo,
) -> String {
    let mut output = String::new();
    output.push_str(&format_cpu_info(cpu_info, cpu_gflops, cpu_elapsed_time));

    let supports_mps = cpu_info.arch == Some("arm64".to_string()) && cpu_info.os == "macos";
    if supports_mps {
        output.push_str(&format_mps_gpu_info());
    } else {
        output.push_str(&format_cuda_gpus_info());
    }
    output.push_str(&format_battery_info(battery_info));
    output
}

fn format_cpu_info(
    cpu_info: &info::cpu::CpuInfo,
    cpu_gflops: f64,
    cpu_elapsed_time: f64,
) -> String {
    format!(
        "=> CPU Bench:\n\
        OS          : {:?}\n\
        OS version  : {}\n\
        Memory Total: {} mb\n\
        Memory Used: {} mb\n\
        Swap Total : {} mb\n\
        Swap Used  : {} mb\n\
        CPU architecture: {:?}\n\
        CPU count   : {}\n\
        CPU FLOPS   : {:.2} GFLOPS\n\
        CPU benchmark duration: {:.2} seconds\n\n",
        cpu_info.os,
        cpu_info.os_version.as_deref().unwrap_or("Not available"),
        cpu_info.total_memory,
        cpu_info.used_memory,
        cpu_info.total_swap,
        cpu_info.used_swap,
        cpu_info.arch.as_deref().unwrap_or("Not available"),
        cpu_info.cpu_count,
        cpu_gflops,
        cpu_elapsed_time
    )
}

fn format_battery_info(battery_info: &BatteryInfo) -> String {
    let charge = if battery_info.charge_percent.is_some() {
        battery_info.charge_percent.unwrap().to_string()
    } else {
        "NA".to_string()
    };
    format!(
        "=> Power:\n\
        Battery        : {:?}\n\
        State of charge: {}%\n\n",
        battery_info.has_battery, charge
    )
}

fn benchmark_mps_gpu() -> io::Result<Vec<serde_json::Value>> {
    let (gpu_tflops, gpu_elapsed_time) = benchmark_gpu(Device::Mps, 1000);
    Ok(vec![json!({
        "device": "MPS",
        "tflops": gpu_tflops,
        "duration": gpu_elapsed_time,
    })])
}

fn benchmark_cuda_gpus() -> io::Result<Vec<serde_json::Value>> {
    let gpu_infos = get_gpu_info();
    let mut gpu_results = Vec::new();

    for (index, info) in gpu_infos.into_iter().enumerate() {
        let (gpu_tflops, gpu_elapsed_time) = benchmark_gpu(Device::Cuda(index), 1000);
        gpu_results.push(json!({
            "device_id": info.device_id,
            "device": format!("{:?}", info.device),
            "name": info.name.unwrap_or_else(|| "Not available".to_string()),
            "total_memory": info.total_memory.unwrap_or(0),
            "free_memory": info.free_memory.unwrap_or(0),
            "used_memory": info.used_memory.unwrap_or(0),
            "tflops": gpu_tflops,
            "duration": gpu_elapsed_time,
        }));
    }

    Ok(gpu_results)
}

fn format_mps_gpu_info() -> String {
    let (gpu_tflops, gpu_elapsed_time) = benchmark_gpu(Device::Mps, 1000);
    format!(
        "=> GPU Bench:\n\
        GPU: integrated (MPS)\n\
        GPU FLOPS: {:.2} TFLOPS\n\
        GPU benchmark duration: {:.2} seconds\n",
        gpu_tflops, gpu_elapsed_time
    )
}

fn format_cuda_gpus_info() -> String {
    let gpu_infos = get_gpu_info();
    let mut output = String::new();

    for (index, info) in gpu_infos.into_iter().enumerate() {
        let (gpu_tflops, gpu_elapsed_time) = benchmark_gpu(Device::Cuda(index), 1000);
        output.push_str(&format!(
            "CUDA Device {} Information:\n\
            Device: {:?}\n\
            Name: {}\n\
            Total Memory: {}\n\
            Free Memory: {}\n\
            Used Memory: {}\n\
            GPU Estimated FLOPS: {:.2} TFLOPS\n\
            GPU benchmark duration: {:.2} seconds\n",
            info.device_id,
            info.device,
            info.name.unwrap_or_else(|| "Not available".to_string()),
            info.total_memory.unwrap_or(0),
            info.free_memory.unwrap_or(0),
            info.used_memory.unwrap_or(0),
            gpu_tflops,
            gpu_elapsed_time
        ));
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
