# dozr

`dozr` is a flexible command-line utility for pausing execution, inspired by the familiar `sleep` command.

## Features

-   **Simple Duration Wait:** Pause for a specified duration (e.g., `5s`, `1m30s`).
-   **Randomized Jitter:** Add a random delay on top of the base duration for more natural or distributed waits.
-   **Verbose Output:** Get real-time feedback on the wait progress, including estimated time remaining (ETA).

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

Wait for 5 seconds:

```bash
dozr 5s
```

### Wait with Jitter

Wait for 2 seconds, plus a random duration up to 1 second:

```bash
dozr 2s --jitter 1s
```

### Verbose Output

Get detailed feedback during the wait:

```bash
dozr 10s --verbose
```

Combine with jitter:

```bash
dozr 5s --jitter 2s --verbose
```

## Contributing

Contributions are welcome! Please refer to the `CONTRIBUTING.md` (to be created) for guidelines.

## License

This project is licensed under the MIT License.