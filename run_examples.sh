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

echo "## Basic Usage"
echo "### Waiting for a fixed duration (5s for demonstration)"
echo "dozr 5s"
$DOZR_BIN 5s
echo ""

echo "### Waiting for 1 minute and 30 seconds"
echo "dozr 1m30s"
$DOZR_BIN 1m30s
echo ""

echo "## Waiting with Jitter"
echo "### Wait for 10 seconds, plus a random duration between 0 and 5 seconds"
echo "dozr 10s --jitter 5000ms"
$DOZR_BIN 10s --jitter 5000ms
echo ""

echo "## Verbose Output"
echo "### Wait for 3 seconds with verbose output"
echo "dozr 3s --verbose"
$DOZR_BIN 3s --verbose
echo ""

echo "### Combine verbose output with jitter (2s base, 1s jitter)"
echo "dozr 10s --jitter 10s -v"
$DOZR_BIN 20s --jitter 10s -v
echo ""

echo "## Custom Verbose Update Period"
echo "### Specify a custom update period for verbose messages (10s wait, 2500ms update)"
echo "dozr 10s --verbose 2500ms"
$DOZR_BIN 10s --verbose 2500ms
echo ""

echo "### Set verbose messages to update every 2 seconds (10s wait)"
echo "dozr 10s --verbose 2s"
$DOZR_BIN 10s --verbose 2s
echo ""

echo "## Time Alignment"
echo "### Wait until the next even 5-second mark"
echo "dozr --align 5s"
$DOZR_BIN --align 5s
echo ""

echo "### Wait until the next even 10-second mark, with verbose output"
echo "dozr --align 10s --verbose"
$DOZR_BIN --align 10s --verbose
echo ""

echo "### Combine with verbose output and a custom update period (15s align, 1s update)"
echo "dozr --align 15s --verbose 1s"
$DOZR_BIN --align 15s --verbose 1s
echo ""

echo "## Using dozr in Pipelines"
echo "### Run a command, wait, then run another command, showing dozr's progress"
echo "echo "Starting process...""
echo "dozr 10s -v"
echo "echo "Process complete.""
echo "Starting process..."
$DOZR_BIN 20s -v
echo "Process complete."
echo ""

echo "### Redirect dozr's verbose output to a log file"
LOG_FILE="dozr_progress.log"
echo "dozr 10s --jitter 500ms -v 2> $LOG_FILE"
echo "cat $LOG_FILE"
$DOZR_BIN 10s --jitter 500ms -v 2> "$LOG_FILE"
echo "Content of $LOG_FILE:"
cat "$LOG_FILE"
rm "$LOG_FILE"
echo ""

echo "--- dozr Examples Complete ---"
