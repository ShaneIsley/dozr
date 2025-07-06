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
    echo "$(date +'%H:%M:%S') - START: $cmd"
    $DOZR_BIN $cmd
    echo "$(date +'%H:%M:%S') - END: $cmd"
    echo ""
}

echo "## Basic Usage"
echo "### Waiting for a fixed duration (1s for demonstration)"
run_dozr_example "--duration 1s"

echo "### Waiting for 1 minute and 30 seconds (demonstration with 2s)"
run_dozr_example "--duration 2s"

echo "## Waiting with Jitter"
echo "### Wait for 1 second, plus a random duration between 0 and 0.5 seconds"
run_dozr_example "--duration 1s --jitter 500ms"

echo "## Verbose Output"
echo "### Wait for 3 seconds with adaptive verbose output"
run_dozr_example "--duration 3s --verbose"

echo "### Combine verbose output with jitter (20s base, 10s jitter)"
run_dozr_example "--duration 20s --jitter 10s -v"

echo "### Specify a custom update period for verbose messages (1s wait, 250ms update)"
run_dozr_example "--duration 1s --verbose 250ms"

echo "### Set verbose messages to update every 1 second (2s wait)"
run_dozr_example "--duration 2s --verbose 1s"

echo "## Time Alignment"
echo "### Wait until the next even 5-second mark"
run_dozr_example "--align 5s"

echo "### Wait until the next even 10-second mark, with verbose output"
run_dozr_example "--align 10s --verbose"

echo "### Combine with verbose output and a custom update period (15s align, 1s update)"
run_dozr_example "--align 15s --verbose 1s"

echo "## Wait Until a Specific Time"
echo "### Wait until 10 seconds from now (HH:MM:SS format)"
CURRENT_TIME=$(date +"%H:%M:%S")
TARGET_TIME=$(date -v+10S +"%H:%M:%S")
echo "Current time: $CURRENT_TIME, Target time: $TARGET_TIME"
run_dozr_example "--until $TARGET_TIME"

echo "### Wait until 5 seconds from now with verbose output (HH:MM format)"
CURRENT_TIME=$(date +"%H:%M")
TARGET_TIME=$(date -v+5S +"%H:%M")
echo "Current time: $CURRENT_TIME, Target time: $TARGET_TIME"
run_dozr_example "--until $TARGET_TIME --verbose"

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

echo "--- dozr Examples Complete ---"
