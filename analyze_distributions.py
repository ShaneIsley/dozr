import argparse
import matplotlib.pyplot as plt
import numpy as np
import sys

def analyze_and_plot(data, distribution_name, params):
    """Analyzes data and plots a histogram."""
    data = np.array(data)

    print(f"--- {distribution_name} Distribution Analysis ---")
    print(f"Parameters: {params}")
    print(f"Number of samples: {len(data)}")
    print(f"Mean: {np.mean(data):.4f}")
    print(f"Median: {np.median(data):.4f}")
    print(f"Standard Deviation: {np.std(data):.4f}")
    print(f"Min: {np.min(data):.4f}")
    print(f"Max: {np.max(data):.4f}")
    print("-" * 40)

    plt.figure(figsize=(10, 6))
    plt.hist(data, bins=50, density=True, alpha=0.7, color='blue', edgecolor='black')
    plt.title(f'Histogram of {distribution_name} Distribution (N={len(data)})')
    plt.xlabel('Value (seconds)')
    plt.ylabel('Probability Density')
    plt.grid(axis='y', alpha=0.75)
    plt.show()

if __name__ == "__main__" :
    parser = argparse.ArgumentParser(description="Analyze and plot distribution samples.")
    parser.add_argument("--distribution", required=True, help="Name of the distribution (e.g., normal, exponential).")
    parser.add_argument("--params", required=True, help="Parameters of the distribution (e.g., 'mean=1.0,std_dev=0.1').")
    args = parser.parse_args()

    print("Paste the output from dist_sampler. Press Ctrl+D (Unix) or Ctrl+Z (Windows) when done.")
    
    samples = []
    for line in sys.stdin:
        try:
            samples.append(float(line.strip()))
        except ValueError:
            print(f"Warning: Could not parse line as float: {line.strip()}", file=sys.stderr)
            continue

    if samples:
        analyze_and_plot(samples, args.distribution, args.params)
    else:
        print("No samples provided. Exiting.", file=sys.stderr)
