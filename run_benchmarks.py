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
    import psutil
except ImportError as e:
    print(f"Error: {e.name} is required. Install it with:")
    print(f"  pip install {e.name}")
    print("Or with uv:")
    print(f"  uv pip install {e.name}")
    sys.exit(1)


import os

def run_rust_benchmarks(strategy, thread_count):
    """Run the Rust benchmark binary for a specific thread count and yield JSON results."""
    # Map benchmark names to output filenames
    filename_map = {
        "Benchmark 1: Million Tiny Tasks (Fibonacci)": "benchmark_1_fibonacci.png",
        "Benchmark 2: Recursive Task Decomposition (QuickSort)": "benchmark_2_quicksort.png",
        "Benchmark 3: Producer-Consumer (Lock-Free)": "benchmark_3_producer_consumer.png",
        "Benchmark 4a: NAS EP (Embarrassingly Parallel)": "benchmark_4a_nas_ep.png",
        "Benchmark 4b: NAS MG (Multi-Grid)": "benchmark_4b_nas_mg.png",
        "Benchmark 4c: NAS CG (Conjugate Gradient)": "benchmark_4c_nas_cg.png",
        "Batching (Parallel For Auto)": "benchmark_batching.png",
        "Allocation Throughput": "benchmark_allocation.png",
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


def sanitize_filename(name):
    """Sanitize benchmark name for use as a filename."""
    return name.lower().replace(':', '').replace(' ', '_').replace('(', '').replace(')', '').replace('[', '').replace(']', '')


def create_overhead_comparison_graph(benchmark_name, results_list, output_filename, comparison_label):
    """Create a comparison graph with multiple series for overhead analysis (< 10k items)."""
    if not results_list:
        return
    
    plt.figure(figsize=(12, 7))
    
    # Use a color map for distinct lines
    try:
        colormap = matplotlib.colormaps['tab10']
    except AttributeError:
        # Fallback for older matplotlib versions
        colormap = plt.cm.get_cmap('tab10')
    
    # Decide Y axis label based on benchmark type (same for all results)
    if any(x in benchmark_name for x in ["Allocation", "Batching", "Fibonacci", "EP", "QuickSort"]):
        if "QuickSort" in benchmark_name:
            y_label = "Time per Element (ns)"
        else:
            y_label = "Time per Task (ns)"
    elif any(x in benchmark_name for x in ["MG", "CG"]):
        y_label = "Time per Grid/Matrix Unit (ns)"
    else:
        y_label = "Total Time (ms)"
    
    for i, result in enumerate(results_list):
        benchmark_name_actual = result.get('name', benchmark_name)
        data_points = result['data_points']
        if not data_points:
            continue
            
        # Filter for data points with less than 10k items (overhead analysis)
        data_points = [dp for dp in data_points if dp['num_tasks'] < 10000]
        if not data_points:
            continue
            
        system_info = result['system_info']
        label = ""
        if comparison_label == "cores":
            label = f"{system_info['cpu_cores']} Cores"
        elif comparison_label == "strategies":
            label = f"{system_info['pinning_strategy']}"
            
        num_tasks = [dp['num_tasks'] for dp in data_points]
        
        # Calculate Y axis data based on benchmark type
        if y_label == "Total Time (ms)":
            y_data = [dp['time_ms'] for dp in data_points]
        else:
            y_data = [(dp['time_ms'] * 1e6) / dp['num_tasks'] if dp['num_tasks'] > 0 else 0 for dp in data_points]
        
        plt.plot(num_tasks, y_data, '-o', label=label, linewidth=2, markersize=6, color=colormap(i % 10))

    plt.xlabel('Number of Tasks / Items', fontsize=12)
    plt.ylabel(y_label, fontsize=12)
    plt.title(f"{benchmark_name} (Overhead Analysis)\nComparison by {comparison_label.capitalize()}", fontsize=14, fontweight='bold')
    plt.grid(True, alpha=0.3)
    
    # Ensure Y starts at 0 to see scaling efficiency/flatness
    if "ns" in y_label:
        plt.ylim(bottom=0)

    plt.legend()
    
    # Add system info text box
    phys_cores = psutil.cpu_count(logical=False)
    log_cores = psutil.cpu_count(logical=True)
    total_ram = psutil.virtual_memory().total / (1024**3)
    info_text = f"Hardware: {phys_cores} Phys Cores, {log_cores} Log Cores, {total_ram:.2f} GB RAM"
    plt.figtext(0.5, 0.01, info_text, ha='center', fontsize=10, 
                bbox=dict(boxstyle='round', facecolor='wheat', alpha=0.5))
    
    plt.tight_layout()
    plt.subplots_adjust(bottom=0.12)
    
    output_path = Path("docs") / output_filename
    plt.savefig(output_path, dpi=100, bbox_inches='tight')
    plt.close()
    print(f"Done: Saved overhead comparison graph to {output_path}")


def create_comparison_graph(benchmark_name, results_list, output_filename, comparison_label):
    """Create a comparison graph with multiple series."""
    if not results_list:
        return
    
    plt.figure(figsize=(12, 7))
    
    # Use a color map for distinct lines
    try:
        colormap = matplotlib.colormaps['tab10']
    except AttributeError:
        # Fallback for older matplotlib versions
        colormap = plt.cm.get_cmap('tab10')
    
    # Decide Y axis label based on benchmark type (same for all results)
    if any(x in benchmark_name for x in ["Allocation", "Batching", "Fibonacci", "EP", "QuickSort"]):
        if "QuickSort" in benchmark_name:
            y_label = "Time per Element (ns)"
        else:
            y_label = "Time per Task (ns)"
    elif any(x in benchmark_name for x in ["MG", "CG"]):
        y_label = "Time per Grid/Matrix Unit (ns)"
    else:
        y_label = "Total Time (ms)"
    
    for i, result in enumerate(results_list):
        benchmark_name_actual = result.get('name', benchmark_name)
        data_points = result['data_points']
        if not data_points:
            continue
            
        # Filter out data points with less than 10k items
        data_points = [dp for dp in data_points if dp['num_tasks'] >= 10000]
        if not data_points:
            continue
            
        system_info = result['system_info']
        label = ""
        if comparison_label == "cores":
            label = f"{system_info['cpu_cores']} Cores"
        elif comparison_label == "strategies":
            label = f"{system_info['pinning_strategy']}"
            
        num_tasks = [dp['num_tasks'] for dp in data_points]
        
        # Calculate Y axis data based on benchmark type
        if y_label == "Total Time (ms)":
            y_data = [dp['time_ms'] for dp in data_points]
        else:
            y_data = [(dp['time_ms'] * 1e6) / dp['num_tasks'] if dp['num_tasks'] > 0 else 0 for dp in data_points]
        
        plt.plot(num_tasks, y_data, '-o', label=label, linewidth=2, markersize=6, color=colormap(i % 10))

    plt.xlabel('Number of Tasks / Items', fontsize=12)
    plt.ylabel(y_label, fontsize=12)
    plt.title(f"{benchmark_name}\nComparison by {comparison_label.capitalize()}", fontsize=14, fontweight='bold')
    plt.grid(True, alpha=0.3)
    
    # Ensure Y starts at 0 to see scaling efficiency/flatness
    if "ns" in y_label:
        plt.ylim(bottom=0)

    plt.legend()
    
    # Add system info text box
    phys_cores = psutil.cpu_count(logical=False)
    log_cores = psutil.cpu_count(logical=True)
    total_ram = psutil.virtual_memory().total / (1024**3)
    info_text = f"Hardware: {phys_cores} Phys Cores, {log_cores} Log Cores, {total_ram:.2f} GB RAM"
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
    
    # Filter out data points with less than 10k items
    data_points = [dp for dp in data_points if dp['num_tasks'] >= 10000]
    if not data_points:
        return
    
    num_tasks = [dp['num_tasks'] for dp in data_points]
    
    # Decide Y axis data and label based on benchmark type
    if any(x in name for x in ["Allocation", "Batching", "Fibonacci", "EP", "QuickSort"]):
        y_data = [(dp['time_ms'] * 1e6) / dp['num_tasks'] if dp['num_tasks'] > 0 else 0 for dp in data_points]
        if "QuickSort" in name:
            y_label = "Time per Element (ns)"
        else:
            y_label = "Time per Task (ns)"
    elif any(x in name for x in ["MG", "CG"]):
        y_data = [(dp['time_ms'] * 1e6) / dp['num_tasks'] if dp['num_tasks'] > 0 else 0 for dp in data_points]
        y_label = "Time per Grid/Matrix Unit (ns)"
    else:
        y_data = [dp['time_ms'] for dp in data_points]
        y_label = "Total Time (ms)"
    
    plt.figure(figsize=(10, 6))
    plt.plot(num_tasks, y_data, 'b-o', linewidth=2, markersize=8)
    plt.xlabel('Number of Tasks / Items', fontsize=12)
    plt.ylabel(y_label, fontsize=12)
    plt.title(f"{name}\nStrategy: {system_info['pinning_strategy']} | Threads: {system_info['cpu_cores']}", fontsize=14, fontweight='bold')
    plt.grid(True, alpha=0.3)
    
    # Ensure Y starts at 0 to see scaling efficiency/flatness
    if "ns" in y_label:
        plt.ylim(bottom=0)

    
    info_text = f"System: {system_info['cpu_cores']} Threads, {system_info['total_memory_gb']:.2f} GB RAM | {system_info['pinning_strategy']}"
    plt.figtext(0.5, 0.01, info_text, ha='center', fontsize=10, 
                bbox=dict(boxstyle='round', facecolor='wheat', alpha=0.5))
    
    plt.tight_layout()
    plt.subplots_adjust(bottom=0.12)
    
    output_path = Path("docs") / output_filename
    plt.savefig(output_path, dpi=100, bbox_inches='tight')
    plt.close()


def create_overhead_single_graph(benchmark_data, output_filename):
    """Create a standard PNG graph from a single benchmark run for overhead analysis (< 10k items)."""
    name = benchmark_data['name']
    data_points = benchmark_data['data_points']
    system_info = benchmark_data['system_info']
    
    if not data_points:
        return
    
    # Filter for data points with less than 10k items (overhead analysis)
    data_points = [dp for dp in data_points if dp['num_tasks'] < 10000]
    if not data_points:
        return
    
    num_tasks = [dp['num_tasks'] for dp in data_points]
    
    # Decide Y axis data and label based on benchmark type
    if any(x in name for x in ["Allocation", "Batching", "Fibonacci", "EP", "QuickSort"]):
        y_data = [(dp['time_ms'] * 1e6) / dp['num_tasks'] if dp['num_tasks'] > 0 else 0 for dp in data_points]
        if "QuickSort" in name:
            y_label = "Time per Element (ns)"
        else:
            y_label = "Time per Task (ns)"
    elif any(x in name for x in ["MG", "CG"]):
        y_data = [(dp['time_ms'] * 1e6) / dp['num_tasks'] if dp['num_tasks'] > 0 else 0 for dp in data_points]
        y_label = "Time per Grid/Matrix Unit (ns)"
    else:
        y_data = [dp['time_ms'] for dp in data_points]
        y_label = "Total Time (ms)"
    
    plt.figure(figsize=(10, 6))
    plt.plot(num_tasks, y_data, 'r-o', linewidth=2, markersize=8)  # Red color for overhead graphs
    plt.xlabel('Number of Tasks / Items', fontsize=12)
    plt.ylabel(y_label, fontsize=12)
    plt.title(f"{name} (Overhead Analysis)\nStrategy: {system_info['pinning_strategy']} | Threads: {system_info['cpu_cores']}", fontsize=14, fontweight='bold')
    plt.grid(True, alpha=0.3)
    
    # Ensure Y starts at 0 to see scaling efficiency/flatness
    if "ns" in y_label:
        plt.ylim(bottom=0)

    
    info_text = f"System: {system_info['cpu_cores']} Threads, {system_info['total_memory_gb']:.2f} GB RAM | {system_info['pinning_strategy']}"
    plt.figtext(0.5, 0.01, info_text, ha='center', fontsize=10, 
                bbox=dict(boxstyle='round', facecolor='wheat', alpha=0.5))
    
    plt.tight_layout()
    plt.subplots_adjust(bottom=0.12)
    
    output_path = Path("docs") / output_filename
    plt.savefig(output_path, dpi=100, bbox_inches='tight')
    plt.close()
    print(f"Done: Saved overhead single graph to {output_path}")


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
    
    # Get core counts
    physical_cores = psutil.cpu_count(logical=False) or (os.cpu_count() // 2) or 1
    logical_cores = psutil.cpu_count(logical=True) or os.cpu_count() or 1
    
    if mode == "compare-cores":
        actual_cores = physical_cores
        print(f"\nMode: Core Comparison (Strategy: Linear)")
        print(f"System has {actual_cores} physical cores.")
        
        targets = [c for c in [1, 4, 8, 16, 24, 32] if c <= actual_cores]
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
            safe_name = sanitize_filename(name)
            filename = f"comparison_cores_{safe_name}.png"
            create_comparison_graph(name, results, filename, "cores")
            # Also create overhead graphs
            overhead_filename = f"comparison_cores_{safe_name}_overhead.png"
            create_overhead_comparison_graph(name, results, overhead_filename, "cores")

    elif mode == "compare-strategies":
        actual_cores = logical_cores
        if actual_cores < 16:
            print(f"Error: Strategy comparison requires at least 16 cores. System has {actual_cores} logical cores.")
            sys.exit(1)
            
        print(f"\nMode: Strategy Comparison (Cores: {actual_cores} logical)")
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
            safe_name = sanitize_filename(name)
            filename = f"comparison_strategies_{safe_name}.png"
            create_comparison_graph(name, results, filename, "strategies")
            # Also create overhead graphs
            overhead_filename = f"comparison_strategies_{safe_name}_overhead.png"
            create_overhead_comparison_graph(name, results, overhead_filename, "strategies")
    
    else: # single mode
        actual_cores = logical_cores
        strategy = sys.argv[1] if len(sys.argv) > 1 else "linear"
        threads = int(sys.argv[2]) if len(sys.argv) > 2 else actual_cores
        
        if threads > actual_cores:
            print(f"Warning: Requested {threads} threads exceeds system capacity ({actual_cores}).")
            
        print(f"\nMode: Single Run (Strategy: {strategy}, Threads: {threads})")
        for result in run_rust_benchmarks(strategy, threads):
            name = result['name']
            safe_name = sanitize_filename(name)
            filename = f"single_{safe_name}_{threads}c_{strategy}.png"
            create_single_graph(result, filename)
            print(f"Done: {name} -> docs/{filename}")
            # Also create overhead graphs
            overhead_filename = f"single_{safe_name}_{threads}c_{strategy}_overhead.png"
            create_overhead_single_graph(result, overhead_filename)

    print("\n" + "=" * 60)
    print("All benchmarks and graphs completed!")
    print("=" * 60)


if __name__ == "__main__":
    main()
