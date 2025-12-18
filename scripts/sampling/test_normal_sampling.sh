#!/bin/bash

# This script tests dozr's Normal distribution sampling and demonstrates its behavior.

# Ensure dozr is built
echo "Building dozr..."
cargo build --release &>/dev/null
if [ $? -ne 0 ]; then
    echo "Error: dozr build failed. Exiting."
    exit 1
fi
DOZR_BIN="./target/release/dozr"

echo "--- Testing dozr Normal Distribution Sampling ---"

# Normal distribution parameters
MEAN_MS=300
STD_DEV=0.05
NUM_SAMPLES=1000
THEORETICAL_MEAN=$(awk "BEGIN {print $MEAN_MS / 1000}")

echo "Normal Parameters: Mean = ${MEAN_MS}ms, Std Dev = ${STD_DEV}"
echo "Theoretical Mean: ${THEORETICAL_MEAN} seconds"
echo "Generating $NUM_SAMPLES samples..."

SAMPLES=()
for i in $(seq 1 $NUM_SAMPLES); do
    SAMPLE_VALUE=$("./target/release/dist_sampler" --distribution normal --mean ${MEAN_MS}ms --std-dev $STD_DEV --count 1)
    SAMPLES+=($SAMPLE_VALUE)
done

echo "Collected Samples: ${SAMPLES[@]}"

# Calculate statistics using Python for floating-point arithmetic
python3 -c "
import sys
samples = [float(x) for x in sys.argv[1:]]
if not samples:
    print('No samples collected.')
    sys.exit(1)
print(f'Calculated Mean: {sum(samples) / len(samples):.3f} seconds')
print(f'Minimum Sample: {min(samples):.3f} seconds')
print(f'Maximum Sample: {max(samples):.3f} seconds')
" "${SAMPLES[@]}"

echo "--- Normal Distribution Sampling Test Complete ---"