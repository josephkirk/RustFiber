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


import os

def run_rust_benchmarks(strategy, thread_count):
    """Run the Rust benchmark binary for a specific thread count and yield JSON results."""
    # Map benchmark names to output filenames
    filename_map = {
        "Benchmark 1: Million Tiny Tasks (Fibonacci)": "benchmark_1_fibonacci.png",
        "Benchmark 2: Recursive Task Decomposition (QuickSort)": "benchmark_2_quicksort.png",
        "Benchmark 3: Producer-Consumer Stress Test": "benchmark_3_producer_consumer.png",
        "Benchmark 4a: NAS EP (Embarrassingly Parallel)": "benchmark_4a_nas_ep.png",
        "Benchmark 4b: NAS MG (Multi-Grid)": "benchmark_4b_nas_mg.png",
        "Benchmark 4c: NAS CG (Conjugate Gradient)": "benchmark_4c_nas_cg.png",
    }

    cmd = ["cargo", "run", "--bin", "benchmarks", "--release", "--", strategy, str(thread_count)]

    try:
        process = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1
        )
        
        import threading
        def print_stderr(pipe):
            for line in pipe:
                # Filter out noise, only print specific headers
                if any(x in line for x in ["Testing", "Completed", "Strategy:"]):
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
                yield result
            except json.JSONDecodeError:
                pass
        
        process.wait()
    except Exception as e:
        print(f"Error running benchmarks: {e}")


def create_comparison_graph(benchmark_name, results_list, output_filename, comparison_label):
    """Create a comparison graph with multiple series."""
    if not results_list:
        return
    
    plt.figure(figsize=(12, 7))
    
    # Use a color map for distinct lines
    colormap = plt.cm.get_cmap('tab10')
    
    for i, result in enumerate(results_list):
        data_points = result['data_points']
        if not data_points:
            continue
            
        system_info = result['system_info']
        label = ""
        if comparison_label == "cores":
            label = f"{system_info['cpu_cores']} Cores"
        elif comparison_label == "strategies":
            label = f"{system_info['pinning_strategy']}"
            
        num_tasks = [dp['num_tasks'] for dp in data_points]
        time_ms = [dp['time_ms'] for dp in data_points]
        
        plt.plot(num_tasks, time_ms, '-o', label=label, linewidth=2, markersize=6, color=colormap(i % 10))

    plt.xlabel('Number of Tasks', fontsize=12)
    plt.ylabel('Time (ms)', fontsize=12)
    plt.title(f"{benchmark_name}\nComparison by {comparison_label.capitalize()}", fontsize=14, fontweight='bold')
    plt.grid(True, alpha=0.3)
    plt.legend()
    
    # Add system info text box
    info_text = f"Hardware: {os.cpu_count()} Total Cores"
    plt.figtext(0.5, 0.01, info_text, ha='center', fontsize=10, 
                bbox=dict(boxstyle='round', facecolor='wheat', alpha=0.5))
    
    plt.tight_layout()
    plt.subplots_adjust(bottom=0.12)
    
    output_path = Path("docs") / output_filename
    plt.savefig(output_path, dpi=100, bbox_inches='tight')
    plt.close()
    print(f"Done: Saved comparison graph to {output_path}")


def create_single_graph(benchmark_data, output_filename):
    """Create a standard PNG graph from a single benchmark run."""
    name = benchmark_data['name']
    data_points = benchmark_data['data_points']
    system_info = benchmark_data['system_info']
    
    if not data_points:
        return
    
    num_tasks = [dp['num_tasks'] for dp in data_points]
    time_ms = [dp['time_ms'] for dp in data_points]
    
    plt.figure(figsize=(10, 6))
    plt.plot(num_tasks, time_ms, 'b-o', linewidth=2, markersize=8)
    plt.xlabel('Number of Tasks', fontsize=12)
    plt.ylabel('Time (ms)', fontsize=12)
    plt.title(f"{name}\nStrategy: {system_info['pinning_strategy']} | Threads: {system_info['cpu_cores']}", fontsize=14, fontweight='bold')
    plt.grid(True, alpha=0.3)
    
    info_text = f"System: {system_info['cpu_cores']} Threads, {system_info['total_memory_gb']:.2f} GB RAM | {system_info['pinning_strategy']}"
    plt.figtext(0.5, 0.01, info_text, ha='center', fontsize=10, 
                bbox=dict(boxstyle='round', facecolor='wheat', alpha=0.5))
    
    plt.tight_layout()
    plt.subplots_adjust(bottom=0.12)
    
    output_path = Path("docs") / output_filename
    plt.savefig(output_path, dpi=100, bbox_inches='tight')
    plt.close()


def main():
    """Main entry point."""
    print("=" * 60)
    print("RustFiber Benchmark Suite - Multi-mode Runner")
    print("=" * 60)
    
    docs_dir = Path("docs")
    docs_dir.mkdir(exist_ok=True)
    
    mode = "single"
    if "--compare-cores" in sys.argv:
        mode = "compare-cores"
    elif "--compare-strategies" in sys.argv:
        mode = "compare-strategies"
    
    actual_cores = os.cpu_count() or 1
    
    if mode == "compare-cores":
        print(f"\nMode: Core Comparison (Strategy: Linear)")
        print(f"System has {actual_cores} cores.")
        
        targets = [c for c in [1, 4, 16, 32, 64, 96] if c <= actual_cores]
        print(f"Target core counts: {targets}")
        
        # benchmark_name -> list of results
        comparison_data = {}
        
        for threads in targets:
            for result in run_rust_benchmarks("linear", threads):
                name = result['name']
                if name not in comparison_data:
                    comparison_data[name] = []
                comparison_data[name].append(result)
        
        for name, results in comparison_data.items():
            safe_name = name.lower().replace(':', '').replace(' ', '_').replace('(', '').replace(')', '')
            filename = f"comparison_cores_{safe_name}.png"
            create_comparison_graph(name, results, filename, "cores")

    elif mode == "compare-strategies":
        if actual_cores < 16:
            print(f"Error: Strategy comparison requires at least 16 cores. System has {actual_cores}.")
            sys.exit(1)
            
        print(f"\nMode: Strategy Comparison (Cores: {actual_cores})")
        strategies = ["none", "linear", "avoid-smt", "ccd-isolation", "tiered-spillover"]
        
        # benchmark_name -> list of results
        comparison_data = {}
        
        for strategy in strategies:
            for result in run_rust_benchmarks(strategy, actual_cores):
                name = result['name']
                if name not in comparison_data:
                    comparison_data[name] = []
                comparison_data[name].append(result)
                
        for name, results in comparison_data.items():
            safe_name = name.lower().replace(':', '').replace(' ', '_').replace('(', '').replace(')', '')
            filename = f"comparison_strategies_{safe_name}.png"
            create_comparison_graph(name, results, filename, "strategies")
    
    else: # single mode
        strategy = sys.argv[1] if len(sys.argv) > 1 else "linear"
        threads = int(sys.argv[2]) if len(sys.argv) > 2 else actual_cores
        
        if threads > actual_cores:
            print(f"Warning: Requested {threads} threads exceeds system capacity ({actual_cores}).")
            
        print(f"\nMode: Single Run (Strategy: {strategy}, Threads: {threads})")
        for result in run_rust_benchmarks(strategy, threads):
            name = result['name']
            safe_name = name.lower().replace(':', '').replace(' ', '_').replace('(', '').replace(')', '')
            filename = f"single_{safe_name}_{threads}c_{strategy}.png"
            create_single_graph(result, filename)
            print(f"Done: {name} -> docs/{filename}")

    print("\n" + "=" * 60)
    print("All benchmarks and graphs completed!")
    print("=" * 60)


if __name__ == "__main__":
    main()
