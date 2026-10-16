#!/usr/bin/env bash
# Prints summary statistics for durations sampled exactly as dozr would.
#
# Usage: scripts/sample_stats.sh [--count N] [--seed S] <dozr command and options>
#   scripts/sample_stats.sh --count 20000 ln 1s 500ms
#   scripts/sample_stats.sh n 100ms 50ms -j 20ms
#
# --count and --seed must come before the command. To plot a histogram,
# pipe the dist_sampler example into scripts/analyze_distributions.py instead.
set -euo pipefail

cd "$(dirname "$0")/.."
cargo run --quiet --release --example dist_sampler -- "$@" | python3 -I -c '
import statistics as st
import sys

xs = [float(line) for line in sys.stdin]
if not xs:
    sys.exit("no samples")
print(f"samples  {len(xs)}")
print(f"mean     {st.mean(xs):.4f}s")
print(f"std dev  {st.stdev(xs) if len(xs) > 1 else 0.0:.4f}s")
print(f"min      {min(xs):.4f}s")
print(f"median   {st.median(xs):.4f}s")
print(f"max      {max(xs):.4f}s")
print(f"zero     {sum(x == 0 for x in xs) / len(xs):.1%}")
'
