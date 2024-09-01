# quick-stats

`quick-stats` is a basic tool for quickly assessing a computer's performance and environment

## Features

- Benchmark CPU performance for floating-point operations per second (GFLOPS)
- Benchmark GPU performance for floating-point operations per second (TFLOPS) 
  - Support for CUDA GPUs and Metal Performance Shaders (MPS) for Apple Silicon
- Check if device has a battery and returns state of charge, charging state and capacity

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
- Add support for ROCm
