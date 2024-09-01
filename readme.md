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

## Dependencies

The GPU TFLOPS benchmark requires `libtorch`

### Installation

1. Download `libtorch` from https://pytorch.org/get-started/locally/.
2. Extract the library to a location of your choice
3. Set the following environment variables

##### Linux:

```bash
export LIBTORCH=/path/to/libtorch
export LD_LIBRARY_PATH=${LIBTORCH}/lib:$LD_LIBRARY_PATH
```

##### Windows

```powershell
$Env:LIBTORCH = "X:\path\to\libtorch"
$Env:Path += ";X:\path\to\libtorch\lib"
```

#### macOS + Homebrew

```bash
brew install pytorch jq
export LIBTORCH=$(brew --cellar pytorch)/$(brew info --json pytorch | jq -r '.[0].installed[0].version')
export LD_LIBRARY_PATH=${LIBTORCH}/lib:$LD_LIBRARY_PATH
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
