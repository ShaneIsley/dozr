# Dozr: A flexible `sleep`-like command-line utility

[![Latest Version](https://img.shields.io/crates/v/dozr.svg)](https://crates.io/crates/dozr)
[![License](https://img.shields.io/crates/l/dozr.svg)](https://github.com/ShaneIsley/dozr/blob/main/LICENSE)

`dozr` extends the standard `sleep` command. It pauses execution for a fixed duration, until a specific time of day, or for a duration sampled from a statistical distribution.

## Features

*   **Simple Duration Waits**: Pause for a fixed duration (e.g., `5s`, `100ms`).
*   **Distribution-Based Waits**: Pause for a duration sampled from a variety of statistical distributions, including:
    *   Normal
    *   Exponential
    *   Log-Normal
    *   Pareto
    *   Uniform
    *   Triangular
    *   Gamma
*   **Time-Based Waits**: Pause until a specific time of day (e.g., `22:30:00`).
*   **Alignment**: Align the wait to the next even interval (e.g., `1m`, `30s`).
*   **Jitter**: Add a random duration of jitter to the wait.
*   **Probabilistic Waits**: Wait only with a certain probability.
*   **Verbose Output**: Display a progress bar with the time remaining.

## Installation

Install `dozr` from [crates.io](https://crates.io/crates/dozr) using `cargo`:

```bash
cargo install dozr
```

## Usage

`dozr` is a flexible replacement for the standard `sleep` command.

### `dozr` vs. `sleep`

`sleep` takes a single duration argument (e.g., `sleep 5`). The table below highlights key differences:

| Feature | `sleep` | `dozr` |
| :--- | :--- | :--- |
| **Basic Duration** | `sleep 5` (seconds only) | `dozr d 5s` (supports `s`, `ms`, `m`, `h`, etc.) |
| **Distribution-based** | No | Yes (Normal, Exponential, Log-Normal, Pareto, Uniform, Triangular, Gamma) |
| **Time-based Wait** | No | Yes (`dozr at 22:30`) |
| **Alignment** | No | Yes (`dozr a 1m`) |
| **Jitter** | No | Yes (`dozr d 10s -j 1s`) |
| **Probabilistic Wait** | No | Yes (`dozr d 30s -p 0.5`) |
| **Verbose Output** | No | Yes (`dozr d 10s -v`) |

### Basic Usage

Wait for a fixed duration:

```bash
# Wait for 5 seconds
dozr d 5s
```

### Distribution-Based Waits

Sample a wait from a Normal distribution with a mean of 10 seconds and a standard deviation of 2 seconds:

```bash
# Wait for a duration sampled from a Normal distribution
dozr n 10s 2
```

### Time-Based Waits

Wait until 10:30 PM:

```bash
# Wait until 10:30 PM
dozr at 22:30
```

### Other Options

Add up to 1 second of random jitter to a 10-second wait:

```bash
# Wait for 10 seconds with up to 1 second of jitter
dozr d 10s -j 1s
```

Wait 30 seconds with a 50% chance of waiting:

```bash
# Wait for 30 seconds with a 50% probability
dozr d 30s -p 0.5
```

Display a progress bar while waiting:

```bash
# Wait for 10 seconds with a progress bar
dozr d 10s -v
```

## Command-Line Arguments

### Main Commands

| Full Command | Alias(es) | Arguments | Example |
| :--- | :--- | :--- | :--- |
| `duration` | `d` | `<TIME>` | `dozr d 5s` |
| `normal` | `n` | `<MEAN> <STD_DEV>` | `dozr n 10s 2.5` |
| `exponential`| `e` | `<LAMBDA>` | `dozr e 0.5` |
| `log-normal` | `ln` | `<MEAN> <STD_DEV>` | `dozr ln 1s 0.5` |
| `pareto` | `par` | `<SCALE> <SHAPE>` | `dozr par 1.0 2.0` |
| `uniform` | `u` | `<MIN> <MAX>` | `dozr u 1s 10s` |
| `triangular` | `t` | `<MIN> <MAX> <MODE>`| `dozr t 0.0 10.0 5.0` |
| `gamma` | `g` | `<SHAPE> <SCALE>` | `dozr g 2.0 1.5` |
| `align` | `a`, `ali` | `<INTERVAL>` | `dozr a 1m` |
| `at` | *(none)* | `<HH:MM[:SS]>` | `dozr at 22:30` |

Bare numbers such as standard deviations, triangular values, and Pareto or gamma scales are in seconds. For `log-normal`, `<MEAN>` and `<STD_DEV>` describe the waits themselves, not the underlying normal distribution. Invalid parameters, such as a negative standard deviation or a uniform `<MIN>` above `<MAX>`, are rejected with exit status 2.

If an `at` time falls in a daylight-saving gap, `dozr` waits until the same clock time after the clocks change (02:30 becomes 03:30). If it falls in a repeated hour, the first occurrence that is still in the future is used.

### Global Options

| Full Option | Short | Value | Description |
| :--- | :--- | :--- | :--- |
| `--jitter` | `-j` | `<TIME>` | Adds a random amount of time up to `<TIME>`. Not applicable to `align` or `at`. |
| `--probability`| `-p` | `<FLOAT>` | The chance (0.0 to 1.0) that the wait will actually occur. Applies to every command and keeps any jitter. |
| `--verbose` | `-v` | `[TIME]` | Shows progress. Can take an optional update interval (e.g., `-v 1s`). |
| `--help` | `-h` | *(none)* | Displays the help message for the command. |
| `--version` | `-V` | *(none)* | Displays the application version. |

## License

`dozr` is licensed under the [MIT License](https://github.com/ShaneIsley/dozr/blob/main/LICENSE).
