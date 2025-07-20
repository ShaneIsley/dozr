#!/bin/bash

# This script demonstrates various usage examples of the dozr command-line utility.

# Ensure dozr is built

echo "Building dozr..."
cargo build --release
if [ $? -ne 0 ]; then
    echo "Error: dozr build failed. Exiting."
    exit 1
fi
DOZR_BIN="./target/release/dozr"

echo "--- Starting dozr Examples ---"
echo ""

run_dozr_example() {
    local cmd="$@"
    echo "$(date +'%H:%M:%S') - START: dozr $cmd"
    local start_time=$(date +%s%N)
    $DOZR_BIN $cmd
    local end_time=$(date +%s%N)
    local elapsed_ms=$((($end_time - $start_time) / 1000000))
    echo "$(date +'%H:%M:%S') - END: dozr $cmd (Elapsed: ${elapsed_ms}ms)"
    echo ""
}

run_dozr_example_and_kill() {
    local cmd="$@"
    local kill_after_seconds=5 # Kill after 5 seconds
    echo "$(date +'%H:%M:%S') - START: dozr $cmd (Will kill after ${kill_after_seconds}s)"
    $DOZR_BIN $cmd &
    local pid=$!
    sleep $kill_after_seconds
    kill $pid 2>/dev/null # Kill the process, suppress error if already exited
    echo "$(date +'%H:%M:%S') - END: dozr $cmd (Killed after ${kill_after_seconds}s)"
    echo ""
}

echo "## Basic Usage"
echo "### Waiting for a fixed duration (1s for demonstration)"
run_dozr_example "--duration 1s"

echo "### Waiting for 2 seconds 400 milliseconds"
run_dozr_example "--duration 2s400ms"

echo "## Waiting with Jitter"
echo "### Wait for 1 second, plus a random duration between 0 and 0.5 seconds"
run_dozr_example "--duration 1s --jitter 500ms"

echo "## Verbose Output"
echo "### Wait for 3 seconds with adaptive verbose output"
run_dozr_example "--duration 3s --verbose"

echo "### Combine verbose output with jitter (5s base, 3s jitter)"
run_dozr_example "--duration 5s --jitter 3s -v"

echo "### Specify a custom update period for verbose messages"
run_dozr_example "--duration 9s --verbose 2s"

echo "### Wait for 25 seconds with adaptive verbose output (should show 5s updates)"
run_dozr_example "--duration 25s --verbose"

echo "## Time Alignment"
echo "### Wait until the next even 5-second mark"
run_dozr_example "--align 5s"

echo "### Wait until the next even 10-second mark, with verbose output"
run_dozr_example "--align 10s --verbose"

echo "### Combine with verbose output and a custom update period"
run_dozr_example "--align 15s --verbose 1s"

echo "## Wait Until a Specific Time"
echo "### Wait until 10 seconds from now (HH:MM:SS format)"
CURRENT_TIME=$(date +"%H:%M:%S")
TARGET_TIME=$(date -v+10S +"%H:%M:%S")
echo "Current time: $CURRENT_TIME, Target time: $TARGET_TIME"
run_dozr_example "--until $TARGET_TIME"

echo "### Wait until the next minute with verbose output (HH:MM format)"
CURRENT_TIME=$(date +"%H:%M")
TARGET_TIME=$(date -v+1M +"%H:%M") # Target the next minute
echo "Current time: $CURRENT_TIME, Target time: $TARGET_TIME"
run_dozr_example "--until $TARGET_TIME --verbose"

echo "### Demonstrate --until rollover and early exit (HH:MM format)"
TARGET_TIME="01:00" # A time that has likely passed today
echo "Target time: $TARGET_TIME (will roll over to next day)"
run_dozr_example_and_kill "--until $TARGET_TIME --verbose"

echo "## Probabilistic Delay"
echo "### Wait for 1 second with a 50% chance"
run_dozr_example "--duration 1s --probability 0.5"

echo "### Wait for 1 second with a 100% chance"
run_dozr_example "--duration 1s --probability 1.0"

echo "### Wait for 1 second with a 0% chance"
run_dozr_example "--duration 1s --probability 0.0"

echo "### Combine with verbose output (3s wait, 75% chance)"
run_dozr_example "--duration 3s --probability 0.75 --verbose"

echo "## Using dozr in Pipelines"
echo "### Run a command, wait, then run another command, showing dozr's progress"
echo "$(date +'%H:%M:%S') - START: echo \"Starting process...\""
echo "Starting process..."
echo "$(date +'%H:%M:%S') - END: echo \"Starting process...\""

echo "$(date +'%H:%M:%S') - START: dozr --duration 2s -v"
$DOZR_BIN --duration 2s -v
echo "$(date +'%H:%M:%S') - END: dozr --duration 2s -v"

echo "$(date +'%H:%M:%S') - START: echo \"Process complete.\""
echo "Process complete."
echo "$(date +'%H:%M:%S') - END: echo \"Process complete.\""
echo ""

echo "### Redirect dozr's verbose output to a log file"
LOG_FILE="dozr_progress.log"
echo "$(date +'%H:%M:%S') - START: dozr --duration 1s --jitter 500ms -v 2> $LOG_FILE"
$DOZR_BIN --duration 1s --jitter 500ms -v 2> "$LOG_FILE"
echo "$(date +'%H:%M:%S') - END: dozr --duration 1s --jitter 500ms -v 2> $LOG_FILE"
echo "Content of $LOG_FILE:"
cat "$LOG_FILE"
rm "$LOG_FILE"
echo ""

echo "## Statistical Distribution Waits"
echo "### Normal Distribution (mean 1s, std dev 100ms)"
run_dozr_example "--normal --normal-mean 1s --normal-std-dev 0.05"

echo "### Exponential Distribution (lambda 0.5)"
run_dozr_example "--exponential --exponential-lambda 5.0"

echo "### Log-Normal Distribution (mean 1s, std dev 100ms)"
run_dozr_example "--log-normal --log-normal-mean 200ms --log-normal-std-dev 0.05"

echo "### Pareto Distribution (scale 1s, shape 1.5)"
run_dozr_example "--pareto --pareto-scale 0.2 --pareto-shape 2.0"

echo "### Uniform Distribution (min 1s, max 5s)"
run_dozr_example "--uniform --uniform-min 10ms --uniform-max 100ms"

echo "### Triangular Distribution (min 0.0, max 0.1, mode 0.05)"
run_dozr_example "--triangular --triangular-min 0.0 --triangular-max 0.1 --triangular-mode 0.05"

echo "### Gamma Distribution (shape 2.0, scale 1.0)"
run_dozr_example "--gamma --gamma-shape 2.0 --gamma-scale 1.0"

echo "
NOTE: The Weibull distribution feature has been temporarily removed due to ongoing issues with its implementation and verification. We plan to re-investigate and potentially re-introduce it in a future release."

echo "--- dozr Examples Complete ---"
