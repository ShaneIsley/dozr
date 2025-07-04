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

Use the `--verbose` or `-v` flag to see real-time status updates, including the estimated time remaining (ETA). By default, updates are adaptive (e.g., every 1 second for long waits, 500ms for short waits).

Wait for 30 seconds with verbose output:

```bash
dozr 30s --verbose
```

Combine verbose output with jitter:

```bash
dozr 1m --jitter 10s -v
```

### Custom Verbose Update Period

Specify a custom update period for verbose messages (e.g., every 250 milliseconds):

```bash
dozr 5s --verbose 250ms
```

Set verbose messages to update every 5 seconds:

```bash
dozr 1m --verbose 5s
```

### Time Alignment

Align execution to the next even time interval. This is useful for synchronizing tasks to specific points in time (e.g., on the hour, every 15 minutes).

Wait until the next even 5-minute mark:

```bash
dozr --align 5m
```

Wait until the next even hour, with verbose output:

```bash
dozr --align 1h --verbose
```

Combine with verbose output and a custom update period:

```bash
dozr --align 30m --verbose 1s
```

### Probabilistic Delay

Execute a wait with a given probability. This is useful for simulating intermittent delays or for chaos engineering.

Wait for 5 seconds with a 50% chance:

```bash
dozr 5s --probability 0.5
```

Wait for 10 seconds with a 100% chance (equivalent to `dozr 10s`):

```bash
dozr 10s --probability 1.0
```

Wait for 10 seconds with a 0% chance (will not wait):

```bash
dozr 10s --probability 0.0
```

Combine with verbose output:

```bash
dozr 3s --probability 0.75 --verbose
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