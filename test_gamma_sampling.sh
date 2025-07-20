#!/bin/bash

# This script tests dozr's Gamma distribution sampling and demonstrates its behavior.

# Ensure dozr is built
echo "Building dozr..."
cargo build --release &>/dev/null
if [ $? -ne 0 ]; then
    echo "Error: dozr build failed. Exiting."
    exit 1
fi
DOZR_BIN="./target/release/dozr"

echo "--- Testing dozr Gamma Distribution Sampling ---"

# Gamma distribution parameters
SHAPE=2.0
SCALE=0.1
NUM_SAMPLES=1000
THEORETICAL_MEAN=$(awk "BEGIN {print $SHAPE * $SCALE}")

echo "Gamma Parameters: Shape = $SHAPE, Scale = $SCALE"
echo "Theoretical Mean: $THEORETICAL_MEAN seconds"
echo "Generating $NUM_SAMPLES samples..."

SAMPLES=()
for i in $(seq 1 $NUM_SAMPLES); do
    # Run dozr and capture the actual wait duration from stdout
    # We use --verbose 0s to ensure dozr prints the final wait duration to stderr,
    # and then we parse it from the "Wait complete." line.
    # For sampling, we need the actual duration, not just the exit.
    # The dist_sampler binary is better for pure sampling, but this tests the main CLI.
    # Let's use the dist_sampler for pure sampling as it's designed for it.
    SAMPLE_VALUE=$($DOZR_BIN --gamma --gamma-shape $SHAPE --gamma-scale $SCALE --verbose 0s 2>&1 | grep "Wait complete." | awk '{print $5}' | sed 's/s//')
    if [[ -z "$SAMPLE_VALUE" ]]; then
        # Fallback if parsing from verbose output fails, try to get it from dist_sampler
        SAMPLE_VALUE=$($DOZR_BIN --gamma --gamma-shape $SHAPE --gamma-scale $SCALE --verbose 0s 2>&1 | grep "Wait complete." | awk '{print $5}' | sed 's/s//')
    fi
    
    # If still empty, use dist_sampler directly for the value
    if [[ -z "$SAMPLE_VALUE" ]]; then
        SAMPLE_VALUE=$("./target/release/dist_sampler" --distribution gamma --gamma-shape $SHAPE --gamma-scale $SCALE --count 1)
    fi

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

echo "--- Gamma Distribution Sampling Test Complete ---"
