# quick-stats

`quick-stats` is a basic tool for quickly assessing a computer's performance and environment

## Features

- Benchmark CPU performance for floating-point operations per second (GFLOPS)
- Benchmark GPU performance for floating-point operations per second (TFLOPS)
  - Support for CUDA GPUs and Metal Performance Shaders (MPS) for Apple Silicon
- Check if device has a battery and returns state of charge, charging state and capacity
- Check upload and download speeds, ping and public IP

## Usage

### Basic Usage

Run all available benchmarks and features (default: cpu, gpu, battery, network):

```bash
cargo run --release
```

### CLI Options

The application supports the following command-line options:

```bash
quick-stats [OPTIONS]

OPTIONS:
    -f, --format <FORMAT>           Sets the output format [default: plain] [possible values: plain, json]
    -o, --outputFile <OUTPUT>       Specifies the file to write the output to
    -e, --features <FEATURE>        Select which benchmarks/features to run [default: cpu,gpu,battery,network]
                                     Possible values: cpu, gpu, battery, network (comma-separated)
        --no-benchmark              Skip CPU and GPU benchmarks, only collect device information
        --no-bandwidth              Skip internet speed measurement
    -h, --help                      Print help information
    -V, --version                   Print version information
```

### Examples

**Run only CPU and GPU benchmarks:**
```bash
cargo run --release -- --features cpu,gpu
```

**Get output in JSON format:**
```bash
cargo run --release -- --format json
```

**Save results to a file:**
```bash
cargo run --release -- --outputFile results.txt
```

**Run only network tests and save as JSON:**
```bash
cargo run --release -- --features network --format json --outputFile network_stats.json
```

**Get device information without running benchmarks (fast):**
```bash
cargo run --release -- --no-benchmark
```

**Get only GPU device info without benchmarks:**
```bash
cargo run --release -- --features gpu --no-benchmark --format json
```

**Get network info without bandwidth measurement (faster):**
```bash
cargo run --release -- --features network --no-bandwidth
```

### Feature Details

- **cpu**: CPU benchmark (GFLOPS) and system information (OS, architecture, memory)
- **gpu**: GPU benchmark (TFLOPS) — ONNX Runtime on Windows/Linux (CUDA, DirectML or
  CPU execution provider, reported as `"ep"` in the JSON output), Metal Performance
  Shaders (MPS) on Apple Silicon
- **battery**: Battery status (charge level, charging state, capacity) if available
- **network**: Network diagnostics (ping, public IP, download/upload speeds)

## Dependencies

Two ML backends, selected per platform at compile time:

- **Windows / Linux — ONNX Runtime** (`ort` crate, `load-dynamic`): nothing is
  linked or downloaded at build time; the onnxruntime shared library is located
  at **runtime** via `ORT_DYLIB_PATH`, falling back to a dylib sitting next to
  the executable. Without it the binary still runs — the GPU benchmark reports
  `ep: "unavailable"`. Use onnxruntime **1.23.2** (on Linux, >= 1.23 is
  required: 1.20–1.22 segfault at process exit).
  - CUDA execution provider additionally needs CUDA 12 + cuDNN 9 installed;
  - DirectML (Windows) needs a DX12 GPU;
  - otherwise the benchmark runs on the CPU execution provider.
- **macOS — libtorch** (tch 0.17, libtorch 2.4.0, MPS): `cargo check`/dev
  builds fetch libtorch automatically (`download-libtorch`) when no `LIBTORCH`
  env var is set; release binaries are built by `.dev_ops/publish_mac_binaries.sh`
  against the exact tree LinkMesh bundles (`libtorch-2.4.0-mac-cpu-arm64`).

### Building for Windows/Linux (no libtorch needed)

No environment variables are required to build — onnxruntime is resolved at
runtime:

```bash
cargo build --release

# to run the GPU benchmark, point at an onnxruntime 1.23.2 library:
export ORT_DYLIB_PATH=/path/to/onnxruntime-linux-x64-1.23.2/lib/libonnxruntime.so   # linux
# $Env:ORT_DYLIB_PATH = "X:\path\to\onnxruntime-win-x64-1.23.2\onnxruntime.dll"    # windows
./target/release/quick_stats -e gpu
```

## To do
- Add support for ROCm (for AMD GPUs, via an ONNX Runtime ROCm execution provider)
