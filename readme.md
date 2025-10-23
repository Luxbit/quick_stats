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

### Feature Details

- **cpu**: CPU benchmark (GFLOPS) and system information (OS, architecture, memory)
- **gpu**: GPU benchmark (TFLOPS) - supports CUDA GPUs and Metal Performance Shaders (MPS) for Apple Silicon
- **battery**: Battery status (charge level, charging state, capacity) if available
- **network**: Network diagnostics (ping, public IP, download/upload speeds)

## Dependencies

The GPU TFLOPS benchmark requires `libtorch` for PyTorch C++ bindings.

### Installation

#### Option 1: Manual Installation

1. Download `libtorch` from https://pytorch.org/get-started/locally/
2. Extract the library to a location of your choice
3. Set the following environment variables:

**Linux:**
```bash
export LIBTORCH=/path/to/libtorch
export LD_LIBRARY_PATH=${LIBTORCH}/lib:$LD_LIBRARY_PATH
```

**Windows:**
```powershell
$Env:LIBTORCH = "X:\path\to\libtorch"
$Env:Path += ";X:\path\to\libtorch\lib"
```

**macOS + Homebrew:**
```bash
brew install pytorch jq
export LIBTORCH=$(brew --cellar pytorch)/$(brew info --json pytorch | jq -r '.[0].installed[0].version')
export LD_LIBRARY_PATH=${LIBTORCH}/lib:$LD_LIBRARY_PATH
```

#### Option 2: Use the installation script

For automated setup, run the provided installation script:

```bash
chmod +x install.sh
./install.sh
```

## To do
- Add support for ROCm (for AMD GPUs)
