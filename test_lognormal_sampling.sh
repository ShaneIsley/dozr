#!/bin/bash

# This script tests dozr's Log-Normal distribution sampling and demonstrates its behavior.

# Ensure dozr is built
echo "Building dozr..."
cargo build --release &>/dev/null
if [ $? -ne 0 ]; then
    echo "Error: dozr build failed. Exiting."
    exit 1
fi
DOZR_BIN="./target/release/dozr"

echo "--- Testing dozr Log-Normal Distribution Sampling ---"

# Log-Normal distribution parameters
# These are the parameters for the *underlying Normal distribution*
MU_MS=200 # Mean of the underlying normal distribution (in milliseconds)
SIGMA=0.05 # Standard deviation of the underlying normal distribution

NUM_SAMPLES=1000

# Calculate the theoretical mean of the Log-Normal distribution
# Theoretical Mean = exp(mu + sigma^2 / 2)
# mu needs to be in natural log scale, so convert MU_MS to seconds and then take log
MU_SECS=$(awk "BEGIN {print $MU_MS / 1000.0}")
THEORETICAL_MEAN_LOGNORMAL=$(awk "BEGIN {print exp(log($MU_MS / 1000.0) + ($SIGMA^2) / 2)}")

echo "Log-Normal Parameters: Mu (underlying normal mean) = ${MU_MS}ms, Sigma (underlying normal std dev) = ${SIGMA}"
echo "Theoretical Mean (Log-Normal): ${THEORETICAL_MEAN_LOGNORMAL} seconds"
echo "Generating $NUM_SAMPLES samples..."

SAMPLES=()
for i in $(seq 1 $NUM_SAMPLES); do
    # Pass mu (as a duration string) and sigma to dist_sampler
    SAMPLE_VALUE=$("./target/release/dist_sampler" --distribution log_normal --mean ${MU_MS}ms --std-dev $SIGMA --count 1)
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

echo "--- Log-Normal Distribution Sampling Test Complete ---"