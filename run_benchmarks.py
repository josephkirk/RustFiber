#!/usr/bin/env python3
"""
Benchmark runner script for RustFiber.
This script runs the Rust benchmark binary and generates PNG graphs from the JSON output.

Usage:
    python3 run_benchmarks.py

Or with uv (recommended):
    uv run run_benchmarks.py
"""

import json
import subprocess
import sys
from pathlib import Path

try:
    import matplotlib.pyplot as plt
    import matplotlib
    matplotlib.use('Agg')  # Non-interactive backend
except ImportError:
    print("Error: matplotlib is required. Install it with:")
    print("  pip install matplotlib")
    print("Or with uv:")
    print("  uv pip install matplotlib")
    sys.exit(1)


def run_rust_benchmarks():
    """Run the Rust benchmark binary and capture JSON output."""
    print("Running Rust benchmarks...")
    
    try:
        result = subprocess.run(
            ["cargo", "run", "--bin", "benchmarks", "--release"],
            capture_output=True,
            text=True,
            check=True
        )
        
        # stderr contains the progress output, stdout contains JSON
        print(result.stderr)  # Print progress to console
        
        # Parse JSON from stdout
        benchmark_results = json.loads(result.stdout)
        return benchmark_results
        
    except subprocess.CalledProcessError as e:
        print(f"Error running benchmarks: {e}")
        print(f"stderr: {e.stderr}")
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error parsing JSON output: {e}")
        print(f"stdout: {result.stdout}")
        sys.exit(1)


def create_graph(benchmark_data, output_filename):
    """Create a PNG graph from benchmark data."""
    name = benchmark_data['name']
    data_points = benchmark_data['data_points']
    system_info = benchmark_data['system_info']
    crashed = benchmark_data.get('crashed', False)
    crash_point = benchmark_data.get('crash_point', None)
    
    if not data_points:
        print(f"Warning: No data points for {name}, skipping graph generation")
        return
    
    # Extract data
    num_tasks = [dp['num_tasks'] for dp in data_points]
    time_ms = [dp['time_ms'] for dp in data_points]
    
    # Create figure
    plt.figure(figsize=(10, 6))
    plt.plot(num_tasks, time_ms, 'b-o', linewidth=2, markersize=8)
    plt.xlabel('Number of Tasks', fontsize=12)
    plt.ylabel('Time (ms)', fontsize=12)
    plt.title(name, fontsize=14, fontweight='bold')
    plt.grid(True, alpha=0.3)
    
    # Add system info and crash info as text below the plot
    info_text = f"System: {system_info['cpu_cores']} CPU cores, {system_info['total_memory_gb']:.2f} GB RAM"
    if crashed and crash_point:
        info_text += f"\n! System crashed at {crash_point} tasks"
    
    plt.figtext(0.5, 0.01, info_text, ha='center', fontsize=10, 
                bbox=dict(boxstyle='round', facecolor='wheat', alpha=0.5))
    
    # Adjust layout to prevent text cutoff
    plt.tight_layout()
    plt.subplots_adjust(bottom=0.12)
    
    # Save figure
    output_path = Path("docs") / output_filename
    plt.savefig(output_path, dpi=100, bbox_inches='tight')
    plt.close()
    
    print(f"✓ Saved graph to {output_path}")


def main():
    """Main entry point."""
    print("=" * 60)
    print("RustFiber Benchmark Suite - Graph Generator")
    print("=" * 60)
    print()
    
    # Ensure docs directory exists
    docs_dir = Path("docs")
    docs_dir.mkdir(exist_ok=True)
    
    # Run benchmarks
    results = run_rust_benchmarks()
    
    print()
    print("=" * 60)
    print("Generating graphs...")
    print("=" * 60)
    print()
    
    # Map benchmark names to output filenames
    filename_map = {
        "Benchmark 1: Million Tiny Tasks (Fibonacci)": "benchmark_1_fibonacci.png",
        "Benchmark 2: Recursive Task Decomposition (QuickSort)": "benchmark_2_quicksort.png",
        "Benchmark 3: Producer-Consumer Stress Test": "benchmark_3_producer_consumer.png",
        "Benchmark 4a: NAS EP (Embarrassingly Parallel)": "benchmark_4a_nas_ep.png",
        "Benchmark 4b: NAS MG (Multi-Grid)": "benchmark_4b_nas_mg.png",
        "Benchmark 4c: NAS CG (Conjugate Gradient)": "benchmark_4c_nas_cg.png",
    }
    
    # Generate graphs
    for result in results:
        name = result['name']
        filename = filename_map.get(name, f"benchmark_{name.replace(' ', '_').lower()}.png")
        create_graph(result, filename)
    
    print()
    print("=" * 60)
    print("All graphs generated successfully!")
    print("=" * 60)
    print()
    print("Graphs saved in docs/ folder:")
    for filename in filename_map.values():
        print(f"  - docs/{filename}")
    print()


if __name__ == "__main__":
    main()
