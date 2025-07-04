# dozr Examples

This document provides various examples of how to use the `dozr` command-line utility.

## Basic Usage

### Waiting for a fixed duration

Wait for 10 seconds:

```bash
dozr 10s
```

Wait for 1 minute and 30 seconds:

```bash
dozr 1m30s
```

### Waiting with Jitter

Add a random delay to your wait. The jitter value specifies the *maximum* additional random duration.

Wait for 5 seconds, plus a random duration between 0 and 2 seconds:

```bash
dozr 5s --jitter 2s
```

This can be useful for distributing load or simulating more natural delays in scripts.

### Verbose Output

Use the `--verbose` or `-v` flag to see real-time status updates, including the estimated time remaining (ETA).

Wait for 30 seconds with verbose output:

```bash
dozr 30s --verbose
```

Combine verbose output with jitter:

```bash
dozr 1m --jitter 10s -v
```

### Using `dozr` in Pipelines

Since `dozr` prints its verbose output to `stderr`, it can be easily integrated into shell pipelines without interfering with `stdout`.

Run a command, wait, then run another command, showing `dozr`'s progress:

```bash
echo "Starting process..."
dozr 5s -v
echo "Process complete."
```

Redirect `dozr`'s verbose output to a log file:

```bash
dozr 1m --jitter 5s -v 2> dozr_progress.log
cat dozr_progress.log
```