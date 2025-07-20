#!/bin/bash

# This script tests dozr's Exponential distribution sampling and demonstrates its behavior.

# Ensure dozr is built
echo "Building dozr..."
cargo build --release &>/dev/null
if [ $? -ne 0 ]; then
    echo "Error: dozr build failed. Exiting."
    exit 1
fi
DOZR_BIN="./target/release/dozr"

echo "--- Testing dozr Exponential Distribution Sampling ---"

# Exponential distribution parameters
LAMBDA=5.0 # Mean = 1/LAMBDA = 1/5.0 = 0.2 seconds = 200ms
NUM_SAMPLES=1000
THEORETICAL_MEAN=$(awk "BEGIN {print 1 / $LAMBDA}")

echo "Exponential Parameters: Lambda = ${LAMBDA}"
echo "Theoretical Mean: ${THEORETICAL_MEAN} seconds"
echo "Generating $NUM_SAMPLES samples..."

SAMPLES=()
for i in $(seq 1 $NUM_SAMPLES); do
    SAMPLE_VALUE=$("./target/release/dist_sampler" --distribution exponential --lambda $LAMBDA --count 1)
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

echo "--- Exponential Distribution Sampling Test Complete ---"
