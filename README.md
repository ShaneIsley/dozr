# dozr

`dozr` is a flexible command-line utility for pausing execution, inspired by the familiar `sleep` command.

## Features

-   **Simple Duration Wait:** Pause for a specified duration (e.g., `5s`, `1m30s`).
-   **Randomized Jitter:** Add a random delay on top of the base duration for more natural or distributed waits.
-   **Statistical Distributions:** Wait for a duration sampled from various statistical distributions:
    -   **Normal Distribution:** For waits centered around a mean with a given standard deviation.
    -   **Exponential Distribution:** For modeling inter-arrival times or random events.
    -   **Log-Normal Distribution:** For waits where the logarithm of the duration is normally distributed.
    -   **Pareto Distribution:** For modeling phenomena where a small number of events account for a large proportion of the total.
    -   **Weibull Distribution:** For modeling reliability, failure rates, and extreme value phenomena.
-   **Verbose Output:** Get real-time feedback on the wait progress. When `--verbose` is used without a specified period, `dozr` intelligently adapts the update frequency (see "Adaptive Verbose" below). A custom, fixed update period can also be specified (e.g., `--verbose 250ms`).
-   **Time Alignment:** Align the wait to the next even interval (e.g., `xx:00`, `xx:15`, `xx:30`).
-   **Probabilistic Delay:** Wait for a duration only with a specified probability (0.0-1.0).

## Installation

To install `dozr` from crates.io (once published):

```bash
cargo install dozr
```

Alternatively, to build and run from source:

```bash
git clone https://github.com/ShaneIsley/dozr.git
cd dozr
cargo build --release
./target/release/dozr --help
```

## Usage

### Basic Wait

Wait for 1 second:

```bash
dozr 1s
```

### Wait with Jitter

Wait for 1 second, plus a random duration up to 0.5 seconds:

```bash
dozr 1s --jitter 500ms
```

### Verbose Output

Get detailed feedback during the wait.

-   **Adaptive Verbose:** When `--verbose` is used without a specified period, `dozr` intelligently adapts the update frequency based on the remaining time:
    -   0-20 seconds remaining: updates every 1 second.
    -   21-60 seconds remaining: updates every 5 seconds.
    -   1-5 minutes remaining: updates every 10 seconds.
    -   6-10 minutes remaining: updates every 15 seconds.
    -   Over 10 minutes remaining: updates every 1 minute.

    ```bash
    dozr 3s --verbose
    ```

-   **Fixed Verbose:** To specify a fixed update period (e.g., every 250 milliseconds):

    ```bash
    dozr 10s --verbose 250ms
    ```

Combine with jitter:

```bash
dozr 2s --jitter 1s --verbose
```

### Time Alignment

Wait until the next even 5-second mark:

```bash
dozr --align 5s
```

Wait until the next even 10-second mark with verbose output:

```bash
dozr --align 10s --verbose
```

### Wait Until a Specific Time

Wait until 5:30 PM today (rolls over to tomorrow if time has passed):

```bash
dozr --until 17:30
```

Wait until 9:00 AM tomorrow with verbose output:

```bash
dozr --until 09:00 --verbose
```

### Probabilistic Delay

Wait for 1 second with a 50% chance:

```bash
dozr 1s --probability 0.5
```

Wait for 1 second with a 100% chance (equivalent to `dozr 1s`):

```bash
dozr 1s --probability 1.0
```

Wait for 1 second with a 0% chance (will not wait):

```bash
dozr 1s --probability 0.0
```

### Statistical Distribution Waits

#### Normal Distribution

Wait for a duration sampled from a Normal distribution with a mean of 1 second and a standard deviation of 0.1:

```bash
dozr --normal-mean 1s --normal-std-dev 0.1
```

#### Exponential Distribution

Wait for a duration sampled from an Exponential distribution with a lambda (rate parameter) of 0.5:

```bash
dozr --exponential-lambda 0.5
```

#### Log-Normal Distribution

Wait for a duration sampled from a Log-Normal distribution with a mean of 1 second and a standard deviation of 0.1:

```bash
dozr --log-normal-mean 1s --log-normal-std-dev 0.1
```

#### Pareto Distribution

Wait for a duration sampled from a Pareto distribution with a scale of 1.0 and a shape of 1.5:

```bash
dozr --pareto-scale 1.0 --pareto-shape 1.5
```

#### Weibull Distribution

Wait for a duration sampled from a Weibull distribution with a shape of 1.5 and a scale of 1.0:

```bash
dozr --weibull-shape 1.5 --weibull-scale 1.0
```

#### Uniform Distribution

Wait for a duration sampled from a Uniform distribution between 1 second and 5 seconds:

```bash
dozr --uniform-min 1s --uniform-max 5s
```

#### Triangular Distribution

Wait for a duration sampled from a Triangular distribution with a minimum of 0.0, a maximum of 1.0, and a mode of 0.5:

```bash
dozr --triangular-min 0.0 --triangular-max 1.0 --triangular-mode 0.5
```

#### Gamma Distribution

Wait for a duration sampled from a Gamma distribution with a shape of 2.0 and a scale of 1.0:

```bash
dozr --gamma-shape 2.0 --gamma-scale 1.0
```



## Contributing

Contributions are welcome! Please refer to the `CONTRIBUTING.md` (to be created) for guidelines.

## License

This project is licensed under the MIT License.