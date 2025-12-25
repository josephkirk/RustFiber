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

# Ensure UTF-8 output on all systems
if sys.stdout.encoding != 'utf-8':
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8')

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
    """Run the Rust benchmark binary and yield JSON results line-by-line."""
    print("Running Rust benchmarks...")
    
    # Map benchmark names to output filenames
    filename_map = {
        "Benchmark 1: Million Tiny Tasks (Fibonacci)": "benchmark_1_fibonacci.png",
        "Benchmark 2: Recursive Task Decomposition (QuickSort)": "benchmark_2_quicksort.png",
        "Benchmark 3: Producer-Consumer Stress Test": "benchmark_3_producer_consumer.png",
        "Benchmark 4a: NAS EP (Embarrassingly Parallel)": "benchmark_4a_nas_ep.png",
        "Benchmark 4b: NAS MG (Multi-Grid)": "benchmark_4b_nas_mg.png",
        "Benchmark 4c: NAS CG (Conjugate Gradient)": "benchmark_4c_nas_cg.png",
    }

    try:
        process = subprocess.Popen(
            ["cargo", "run", "--bin", "benchmarks", "--release"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1
        )
        
        # In a separate thread or just by alternating, we should handle stderr
        # but for simplicity in this script, we'll focus on stdout lines.
        # The Rust binary prints mostly to stderr for progress.
        
        import threading
        def print_stderr(pipe):
            for line in pipe:
                print(line, end='', file=sys.stderr)
        
        stderr_thread = threading.Thread(target=print_stderr, args=(process.stderr,))
        stderr_thread.daemon = True
        stderr_thread.start()

        for line in process.stdout:
            line = line.strip()
            if not line:
                continue
            
            try:
                result = json.loads(line)
                name = result['name']
                system_info = result['system_info']
                cores = system_info['cpu_cores']
                ram = round(system_info['total_memory_gb'])
                
                # Get base filename
                base_filename = filename_map.get(name, f"benchmark_{name.replace(' ', '_').lower()}")
                # Remove .png from base if present
                if base_filename.endswith(".png"):
                    base_filename = base_filename[:-4]
                
                filename = f"{base_filename}_{cores}c_{ram}gb.png"
                
                print(f"\nCompleted: {name}")
                if result.get('timed_out'):
                    print(f"  ! Note: Benchmark timed out after 1 minute")
                
                create_graph(result, filename)
                yield name, filename
            except json.JSONDecodeError as e:
                print(f"Error parsing JSON line: {e}")
                print(f"Line content: {line}")
        
        process.wait()
        if process.returncode != 0:
            print(f"Benchmarks process exited with code {process.returncode}")
            
    except Exception as e:
        print(f"Error running benchmarks: {e}")
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
    
    # Add timeout info if applicable
    if benchmark_data.get('timed_out'):
        info_text += "\n! Benchmark timed out after 1 minute"

    plt.figtext(0.5, 0.01, info_text, ha='center', fontsize=10, 
                bbox=dict(boxstyle='round', facecolor='wheat', alpha=0.5))
    
    # Adjust layout to prevent text cutoff
    plt.tight_layout()
    plt.subplots_adjust(bottom=0.12)
    
    # Save figure
    output_path = Path("docs") / output_filename
    plt.savefig(output_path, dpi=100, bbox_inches='tight')
    plt.close()
    
    print(f"Done: Saved graph to {output_path}")


def main():
    """Main entry point."""
    print("=" * 60)
    print("RustFiber Benchmark Suite - Graph Generator")
    print("=" * 60)
    print()
    
    # Ensure docs directory exists
    docs_dir = Path("docs")
    docs_dir.mkdir(exist_ok=True)
    
    # Run benchmarks and generate graphs incrementally
    generated_files = []
    for name, filename in run_rust_benchmarks():
        generated_files.append((name, filename))
    
    print()
    print("=" * 60)
    print("All benchmarks and graphs completed!")
    print("=" * 60)
    print()
    print("Graphs saved in docs/ folder:")
    for name, filename in generated_files:
        print(f"  - {name} -> docs/{filename}")
    print()


if __name__ == "__main__":
    main()
