# quick-stats

`quick-stats` is a basic tool for quickly assessing a computer's performance and environment

## Features

- Benchmarks CPU performance with floating-point operations per second (GFLOPS)
- Benchmarks GPU performance with floating-point operations per second (TFLOPS) 
  - Supports CUDA GPUs and Metal Performance Shaders (MPS) for Apple Silicon

## Usage

```bash 
cargo run --release
```


## CLI options

```bash 
-o, --output FORMAT: Sets the output format. Options are plain (default) or json.
-f, --outputFile FILE: Specifies the file to write the output to.
```

## To do

- Add networking
  - speed
  - ping
  - ip
- Add power source monitoring
  - battery
  - state of charge
- Add support for ROCm
