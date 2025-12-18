# dozr Examples

This document provides various examples of how to use the `dozr` command-line utility.

## Basic Usage

### Waiting for a fixed duration

Wait for 10 seconds:

```bash
dozr d 10s
```

Wait for 1 minute and 30 seconds:

```bash
dozr d 1m30s
```

### Waiting with Jitter

Add a random delay to your wait. The jitter value specifies the *maximum* additional random duration.

Wait for 5 seconds, plus a random duration between 0 and 2 seconds:

```bash
dozr d 5s -j 2s
```

This can be useful for distributing load or simulating more natural delays in scripts.

### Verbose Output

Use the `--verbose` or `-v` flag to see real-time status updates, including the estimated time remaining (ETA). By default, updates are adaptive (e.g., every 1 second for long waits, 500ms for short waits).

Wait for 30 seconds with verbose output:

```bash
dozr d 30s -v
```

Combine verbose output with jitter:

```bash
dozr d 1m -j 10s -v
```

### Custom Verbose Update Period

Specify a custom update period for verbose messages (e.g., every 250 milliseconds):

```bash
dozr d 5s -v 250ms
```

Set verbose messages to update every 5 seconds:

```bash
dozr d 1m -v 5s
```

### Time Alignment

Align execution to the next even time interval. This is useful for synchronizing tasks to specific points in time (e.g., on the hour, every 15 minutes).

Wait until the next even 5-second mark:

```bash
dozr a 5s
```

Wait until the next even 10-second mark, with verbose output:

```bash
dozr a 10s -v
```

Combine with verbose output and a custom update period:

```bash
dozr a 15s -v 1s
```

### Wait Until a Specific Time

Wait until a specific time of day:

```bash
dozr at 22:30
```

Wait until 2:30 PM with verbose output:

```bash
dozr at 14:30:00 -v
```

### Probabilistic Delay

Execute a wait with a given probability. This is useful for simulating intermittent delays or for chaos engineering.

Wait for 5 seconds with a 50% chance:

```bash
dozr d 5s -p 0.5
```

Wait for 10 seconds with a 100% chance (equivalent to `dozr d 10s`):

```bash
dozr d 10s -p 1.0
```

Wait for 10 seconds with a 0% chance (will not wait):

```bash
dozr d 10s -p 0.0
```

Combine with verbose output:

```bash
dozr d 3s -p 0.75 -v
```

### Using `dozr` in Pipelines

Since `dozr` prints its verbose output to `stderr`, it can be easily integrated into shell pipelines without interfering with `stdout`.

Run a command, wait, then run another command, showing `dozr`'s progress:

```bash
echo "Starting process..."
dozr d 5s -v
echo "Process complete."
```

Redirect `dozr`'s verbose output to a log file:

```bash
dozr d 1m -j 5s -v 2> dozr_progress.log
cat dozr_progress.log
```

## Statistical Distribution Waits

### Normal Distribution

Wait for a duration sampled from a Normal distribution with a mean of 1 second and a standard deviation of 0.1:

```bash
dozr n 1s 0.1
```

### Exponential Distribution

Wait for a duration sampled from an Exponential distribution with a lambda (rate parameter) of 0.5:

```bash
dozr e 0.5
```

### Log-Normal Distribution

Wait for a duration sampled from a Log-Normal distribution with a mean of 1 second and a standard deviation of 0.5:

```bash
dozr ln 1s 0.5
```

### Pareto Distribution

Wait for a duration sampled from a Pareto distribution with a scale of 1.0 and a shape of 2.0:

```bash
dozr par 1.0 2.0
```

### Uniform Distribution

Wait for a duration sampled from a Uniform distribution between 1 second and 5 seconds:

```bash
dozr u 1s 5s
```

### Triangular Distribution

Wait for a duration sampled from a Triangular distribution with a minimum of 0.0, a maximum of 10.0, and a mode of 5.0:

```bash
dozr t 0.0 10.0 5.0
```

### Gamma Distribution

Wait for a duration sampled from a Gamma distribution with a shape of 2.0 and a scale of 1.5:

```bash
dozr g 2.0 1.5
```

## Library Usage

For programmatic usage, see the runnable examples in the `examples/` directory:

```bash
# Basic duration waits
cargo run --example basic_wait

# Statistical distribution sampling
cargo run --example distributions

# Verbose progress output
cargo run --example verbose_progress
```
