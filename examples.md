# dozr Examples

This document provides various examples of how to use the `dozr` command-line utility.

## Basic Usage

### Waiting for a fixed duration

Wait for 10 seconds:

```bash
dozr --duration 10s
```

Wait for 1 minute and 30 seconds:

```bash
dozr --duration 1m30s
```

### Waiting with Jitter

Add a random delay to your wait. The jitter value specifies the *maximum* additional random duration.

Wait for 5 seconds, plus a random duration between 0 and 2 seconds:

```bash
dozr --duration 5s --jitter 2s
```

This can be useful for distributing load or simulating more natural delays in scripts.

### Verbose Output

Use the `--verbose` or `-v` flag to see real-time status updates, including the estimated time remaining (ETA). By default, updates are adaptive (e.g., every 1 second for long waits, 500ms for short waits).

Wait for 30 seconds with verbose output:

```bash
dozr --duration 30s --verbose
```

Combine verbose output with jitter:

```bash
dozr --duration 1m --jitter 10s -v
```

### Custom Verbose Update Period

Specify a custom update period for verbose messages (e.g., every 250 milliseconds):

```bash
dozr --duration 5s --verbose 250ms
```

Set verbose messages to update every 5 seconds:

```bash
dozr --duration 1m --verbose 5s
```

### Time Alignment

Align execution to the next even time interval. This is useful for synchronizing tasks to specific points in time (e.g., on the hour, every 15 minutes).

Wait until the next even 5-second mark:

```bash
dozr --align 5s
```

Wait until the next even 10-second mark, with verbose output:

```bash
dozr --align 10s --verbose
```

Combine with verbose output and a custom update period:

```bash
dozr --align 15s --verbose 1s
```

### Probabilistic Delay

Execute a wait with a given probability. This is useful for simulating intermittent delays or for chaos engineering.

Wait for 5 seconds with a 50% chance:

```bash
dozr --duration 5s --probability 0.5
```

Wait for 10 seconds with a 100% chance (equivalent to `dozr 10s`):

```bash
dozr --duration 10s --probability 1.0
```

Wait for 10 seconds with a 0% chance (will not wait):

```bash
dozr --duration 10s --probability 0.0
```

Combine with verbose output:

```bash
dozr --duration 3s --probability 0.75 --verbose
```

### Using `dozr` in Pipelines

Since `dozr` prints its verbose output to `stderr`, it can be easily integrated into shell pipelines without interfering with `stdout`.

Run a command, wait, then run another command, showing `dozr`'s progress:

```bash
echo "Starting process..."
dozr --duration 5s -v
echo "Process complete."
```

Redirect `dozr`'s verbose output to a log file:

```bash
dozr --duration 1m --jitter 5s -v 2> dozr_progress.log
cat dozr_progress.log
```

## Statistical Distribution Waits

### Normal Distribution

Wait for a duration sampled from a Normal distribution with a mean of 1 second and a standard deviation of 100 milliseconds:

```bash
dozr --normal-mean 1s --normal-std-dev 100ms
```

### Exponential Distribution

Wait for a duration sampled from an Exponential distribution with a lambda (rate parameter) of 0.5:

```bash
dozr --exponential-lambda 0.5
```

### Log-Normal Distribution

Wait for a duration sampled from a Log-Normal distribution with a mean of 1 second and a standard deviation of 100 milliseconds:

```bash
dozr --log-normal-mean 1s --log-normal-std-dev 100ms
```

### Pareto Distribution

Wait for a duration sampled from a Pareto distribution with a scale of 1 second and a shape of 1.5:

```bash
dozr --pareto-scale 1s --pareto-shape 1.5
```



### Uniform Distribution

Wait for a duration sampled from a Uniform distribution between 1 second and 5 seconds:

```bash
dozr --uniform-min 1s --uniform-max 5s
```

### Triangular Distribution

Wait for a duration sampled from a Triangular distribution with a minimum of 0.0, a maximum of 1.0, and a mode of 0.5:

```bash
dozr --triangular-min 0.0 --triangular-max 1.0 --triangular-mode 0.5
```

### Gamma Distribution

Wait for a duration sampled from a Gamma distribution with a shape of 2.0 and a scale of 1.0:

```bash
dozr --gamma-shape 2.0 --gamma-scale 1.0
```


