# dozr

`dozr` is a flexible command-line utility for pausing execution, inspired by the familiar `sleep` command.

## Features

-   **Simple Duration Wait:** Pause for a specified duration (e.g., `5s`, `1m30s`).
-   **Randomized Jitter:** Add a random delay on top of the base duration for more natural or distributed waits.
-   **Verbose Output:** Get real-time feedback on the wait progress. By default (`--verbose`), updates are printed every 1 second. A custom update period can also be specified (e.g., `--verbose 250ms`).
-   **Time Alignment:** Align the wait to the next even interval (e.g., `xx:00`, `xx:15`, `xx:30`).
-   **Probabilistic Delay:** Wait for a duration only with a specified probability (0.0-1.0).

## Installation

To install `dozr` from crates.io (once published):

```bash
cargo install dozr
```

Alternatively, to build and run from source:

```bash
git clone https://github.com/your-username/dozr.git # Replace with actual repo URL
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

Get detailed feedback during the wait. When `--verbose` is used without a specified period, `dozr` intelligently adapts the update frequency based on the remaining time (e.g., less frequent for long waits, more frequent as the end approaches).

```bash
dozr 3s --verbose
```

To specify a fixed update period (e.g., every 250 milliseconds):

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

## Contributing

Contributions are welcome! Please refer to the `CONTRIBUTING.md` (to be created) for guidelines.

## License

This project is licensed under the MIT License.