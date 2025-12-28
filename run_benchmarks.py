#!/usr/bin/env python3
"""
Benchmark runner script for RustFiber.
Runs benchmarks and generates high-quality visualization graphs.

Usage:
    uv run run_benchmarks.py [--compare-cores | --compare-strategies | strategy_name] [threads] [--trace]
"""

import json
import subprocess
import sys
import os
from pathlib import Path
import math

# Ensure UTF-8 output
if sys.stdout.encoding != 'utf-8':
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8')

try:
    import matplotlib.pyplot as plt
    import matplotlib.ticker as ticker
    import matplotlib
    matplotlib.use('Agg')
    import psutil
    setup_matplotlib = True
except ImportError as e:
    setup_matplotlib = False
    print(f"Warning: {e.name} is missing. Visualization graphs will not be generated.")
    print("Install with: uv pip install matplotlib psutil")

# --- Configuration ---
COLORS = ['#4C72B0', '#DD8452', '#55A868', '#C44E52', '#8172B3', '#937860', '#DA8BC3', '#8C8C8C']
MARKERS = ['o', 's', '^', 'D', 'v', '<', '>', 'p']
LINE_WIDTH = 2.5
MARKER_SIZE = 8
GRID_ALPHA = 0.4

def setup_style():
    """Configure matplotlib style for professional output."""
    try:
        plt.style.use('seaborn-v0_8-darkgrid')
    except OSError:
        plt.style.use('ggplot')
    
    plt.rcParams.update({
        'font.size': 12,
        'axes.labelsize': 14,
        'axes.titlesize': 16,
        'xtick.labelsize': 11,
        'ytick.labelsize': 11,
        'legend.fontsize': 11,
        'figure.figsize': (12, 8),
        'axes.grid': True,
        'grid.alpha': GRID_ALPHA,
        'lines.linewidth': LINE_WIDTH,
        'lines.markersize': MARKER_SIZE,
    })

def get_sweep_counts(max_threads):
    """Generate a sensible core count sweep up to max_threads."""
    topo_counts = [1, 2, 4, 8, 12, 16, 24, 32, 48, 64, 96, 128]
    sweep = [c for c in topo_counts if c < max_threads]
    sweep.append(max_threads)
    return sorted(list(set(sweep)))

def run_rust_benchmarks(strategy, thread_count, use_tracing=True):
    """Run cargo benchmarks and yield parsed JSON objects."""
    features = "metrics tracing" if use_tracing else "metrics"
        
    cmd = ["cargo", "run", "--bin", "benchmarks", "--release", "--features", features, "--", strategy, str(thread_count), "metrics"]
    
    try:
        process = subprocess.Popen(
            cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1
        )
        
        # Forward stderr in real-time
        import threading
        def print_stderr(pipe):
            for line in pipe:
                if any(x in line for x in ["Testing", "Completed", "Strategy:", "Initializing", "startup", "All Benchmarks"]):
                    print(line, end='', file=sys.stderr)
        
        t = threading.Thread(target=print_stderr, args=(process.stderr,), daemon=True)
        t.start()

        for line in process.stdout:
            line = line.strip()
            if not line: continue
            try:
                yield json.loads(line)
            except json.JSONDecodeError:
                pass
        process.wait()
    except Exception as e:
        print(f"Failed to run benchmarks: {e}")

def sanitize_filename(name):
    return name.lower().replace(':', '').replace(' ', '_').replace('(', '').replace(')', '').replace('/', '_')

def get_metric_config(benchmark_name, result_data_points):
    """Determine axis labels, scaling, and formatting based on data."""
    config = {
        'ylabel': 'Total Time (ms)',
        'log_x': True,
        'log_y': False,
        'transform_y': lambda ms, n: ms
    }

    metric_type = None
    if result_data_points and 'metric_type' in result_data_points[0]:
        metric_type = result_data_points[0]['metric_type']

    if "ParallelFor Scaling" in benchmark_name or metric_type in ['scaling', 'scaling_factor']:
        config['ylabel'] = 'Speedup Factor (x)'
        config['log_y'] = False
        config['log_x'] = False 
        config['transform_y'] = lambda ms, n: ms
        config['is_speedup'] = True
    elif "Throughput" in benchmark_name:
        config['ylabel'] = 'Throughput (Tasks/sec)'
        config['log_y'] = True
        config['transform_y'] = lambda ms, n: (n / (ms / 1000.0)) if ms > 0 else 0
    elif any(x in benchmark_name for x in ["Latency", "Allocation", "Fibonacci", "QuickSort"]):
        config['ylabel'] = 'Latency per Task (μs)'
        config['log_y'] = False 
        config['transform_y'] = lambda ms, n: ms * 1000  # Convert ms to μs
    elif "Transform" in benchmark_name:
        config['ylabel'] = 'Time per Update (ms)'
        config['log_y'] = False
        config['transform_y'] = lambda ms, n: ms / n if n > 0 else 0
    elif "Stress" in benchmark_name:
        # Check if this has steal metrics
        has_steal_metrics = any(d.get('metric_type', '').endswith('_steals_success') or 
                               d.get('metric_type', '').endswith('_steals_failed') or
                               d.get('metric_type', '').endswith('_steals_retry')
                               for d in result_data_points)
        
        # Always use the steal chart (categorical thread columns) if metrics are present
        if has_steal_metrics:
            config['ylabel'] = 'Steal Attempts'
            config['log_y'] = False
            config['log_x'] = False
            config['transform_y'] = lambda ms, n: ms  # ms field contains the count
            config['is_steal_chart'] = True
        else:
            config['ylabel'] = 'Total Execution Time (ms)'
            config['log_y'] = False
            config['log_x'] = False
            config['transform_y'] = lambda ms, n: ms
        
    return config

def plot_graph(title, results_list, output_filename, mode="single"):
    """Generic plotting function for single or comparison graphs."""
    if not results_list: return

    setup_style()
    
    first_result = results_list[0]
    config = get_metric_config(first_result.get('name', title), first_result['data_points'])

    # Determine if we should use Overlay (X=Workload) or Scaling (X=Threads)
    all_workloads = set()
    for res in results_list:
        if res.get('data_points'):
            for dp in res['data_points']:
                all_workloads.add(dp['num_tasks'])
    
    is_fixed_workload = len(all_workloads) == 1
    
    # We use Scaling View (X=Threads) if there is only one workload size
    # OR if explicitly requested by the benchmark type (though usually it's workload size driven)
    use_scaling_view = is_fixed_workload and len(results_list) > 1

    if config.get('is_steal_chart', False):
        # --- CATEGORICAL MODE: X = Thread Count, Y = Steal Attempts ---
        # Split into two subplots: Top = Stacked Bars (Log), Bottom = Success Rate (Linear)
        fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 10), sharex=True, gridspec_kw={'height_ratios': [2, 1]})
        
        # --- Top Plot: Steal Attempts (Log) ---
        ax1.set_ylabel('Steal Attempts (Log Scale)')
        ax1.set_yscale('log')
        ax1.set_title(title, pad=10)
        
        # Process data
        thread_counts = []
        successful_steals, failed_steals, retry_steals, success_rates = [], [], [], []
        
        results_list.sort(key=lambda x: x['system_info']['cpu_cores'])
        
        for res in results_list:
            sys_info = res['system_info']
            thread_count = sys_info['cpu_cores']
            thread_counts.append(thread_count)
            
            ts, tf, tr = 0, 0, 0
            for dp in res['data_points']:
                metric_type = dp.get('metric_type', '')
                count = int(dp['time_ms'])
                if metric_type.endswith('_steals_success'): ts += count
                elif metric_type.endswith('_steals_failed'): tf += count
                elif metric_type.endswith('_steals_retry'): tr += count
            
            successful_steals.append(ts)
            failed_steals.append(tf)
            retry_steals.append(tr)
            total = ts + tf + tr
            success_rates.append((ts / total * 100) if total > 0 else 0)
            
        x_pos = range(len(thread_counts))
        
        # Stacked Bars on ax1
        p1 = ax1.bar(x_pos, failed_steals, 0.6, label='Empty Attempts', color='#ff6b6b', alpha=0.8, log=True)
        p2 = ax1.bar(x_pos, retry_steals, 0.6, bottom=failed_steals, label='Retry (Contention)', color='#ffd93d', alpha=0.8, log=True)
        bottom_success = [f+r for f,r in zip(failed_steals, retry_steals)]
        p3 = ax1.bar(x_pos, successful_steals, 0.6, bottom=bottom_success, label='Successful Steals', color='#4ecdc4', alpha=0.8, log=True)
        
        ax1.legend(frameon=True, framealpha=1.0, loc='upper left')
        ax1.grid(True, which="both", ls="-", alpha=0.2)
        
        # --- Bottom Plot: Success Rate (Linear) ---
        ax2.set_ylabel('Success Rate (%)')
        ax2.set_xlabel('Thread Count')
        ax2.plot(x_pos, success_rates, 'ko-', linewidth=2, markersize=6, label='Success Rate', color='#2c3e50')
        ax2.fill_between(x_pos, success_rates, color='#2c3e50', alpha=0.1)
        
        ax2.set_ylim(0, 105)
        ax2.set_xticks(x_pos)
        ax2.set_xticklabels([f'{tc}T' for tc in thread_counts])
        ax2.grid(True, alpha=0.4)
        
        # Annotate rates on the bottom plot
        for i, rate in enumerate(success_rates):
            ax2.annotate(f'{rate:.1f}%', xy=(i, rate), xytext=(0, 8), textcoords="offset points", 
                         ha='center', va='bottom', fontsize=10, fontweight='bold')
        
    elif use_scaling_view:
        fig, ax = plt.subplots()
        # --- SCALING MODE: X = CPU Threads, Y = Metric ---
        ax.set_xlabel('CPU Threads')
        ax.set_ylabel(config['ylabel'])
        
        results_list.sort(key=lambda x: x['system_info']['cpu_cores'])
        
        x_threads = [r['system_info']['cpu_cores'] for r in results_list]
        y_metric = []
        for r in results_list:
            data = r['data_points']
            if not data:
                y_metric.append(0)
                continue
            dp = data[0]
            val = config['transform_y'](dp['time_ms'], dp['num_tasks'])
            y_metric.append(val)
        
        # IMPROVEMENT: Normalize speedup to 1.0x at 1 thread
        if config.get('is_speedup', False) and len(y_metric) > 0 and y_metric[0] > 0:
            base = y_metric[0]
            y_metric = [y / base for y in y_metric]

        ax.plot(x_threads, y_metric, marker='o', linewidth=LINE_WIDTH, markersize=MARKER_SIZE, color=COLORS[0], label="Measured Scaling")
        
        # Add Ideal Scaling Line (Starting at 1,1)
        if config.get('is_speedup', False):
            ideal_y = [xc for xc in x_threads]
            ax.plot(x_threads, ideal_y, linestyle='--', color='gray', alpha=0.7, label='Ideal Scaling')
            ax.set_ylim(bottom=0)
            ax.legend(frameon=True, framealpha=1.0)
            
        ax.set_title(title, pad=20)

    else:
        fig, ax = plt.subplots()
        # --- OVERLAY MODE: X = Workload Size, Y = Metric (Multiple Lines for Threads) ---
        all_y = []
        # Sort results by core count for cleaner legend
        results_list.sort(key=lambda x: x['system_info']['cpu_cores'])
        
        for i, res in enumerate(results_list):
            data = res['data_points']
            # Include 'time', 'scaling_factor', or 'latency' metrics
            data = [d for d in data if d.get('metric_type') in [None, 'time', 'scaling_factor', 'latency'] and d['time_ms'] > 0]
            if not data: continue

            x = [d['num_tasks'] for d in data]
            y = [config['transform_y'](d['time_ms'], d['num_tasks']) for d in data]
            all_y.extend(y)

            tc = res['system_info']['cpu_cores']
            label = f"{tc}T"
            ax.plot(x, y, marker=MARKERS[i % len(MARKERS)], label=label, color=COLORS[i % len(COLORS)], alpha=0.9)

        if config['log_x']:
            ax.set_xscale('log')
            ax.xaxis.set_major_formatter(ticker.FuncFormatter(lambda val, _: '{:g}'.format(val)))
        
        if config['log_y']: ax.set_yscale('log')
        elif all_y: ax.set_ylim(bottom=0, top=max(all_y) * 1.15)
        
        ax.set_xlabel('Workload Size (Tasks/Items)')
        ax.set_ylabel(config['ylabel'])
        if all_y:
            ax.legend(frameon=True, framealpha=1.0)
            
        ax.set_title(title, pad=20)

    footer = f"Hardware: {psutil.cpu_count(False)}P/{psutil.cpu_count(True)}L Cores, {psutil.virtual_memory().total/1e9:.1f}GB RAM"
    plt.figtext(0.5, 0.02, footer, ha='center', fontsize=10, color='#555555')

    plt.tight_layout()
    plt.subplots_adjust(bottom=0.15)
    
    out_path = Path("docs") / output_filename
    out_path.parent.mkdir(exist_ok=True)
    plt.savefig(out_path, dpi=120)
    plt.close()
    print(f"Generated: {out_path}")

def print_runs_summary(name, results, config):
    """Print the 'scaling summary' for a benchmark."""
    if not results: return
    
    unit = config['ylabel'].split('(')[-1].split(')')[0] if '(' in config['ylabel'] else ""
    
    # Format runs with their thread counts, picking the most representative point for each
    runs_formatted = []
    avg_vals = []
    for res in results:
        data = [d for d in res['data_points'] if d.get('metric_type') in [None, 'time', 'scaling_factor', 'latency']]
        if data:
            # Pick Representative Point: Maximum num_tasks
            dp = max(data, key=lambda d: d['num_tasks'])
            tc = res['system_info']['cpu_cores']
            val = config['transform_y'](dp['time_ms'], dp['num_tasks'])
            runs_formatted.append(f"{tc}T: {val:.2f}")
            avg_vals.append(val)
    
    if avg_vals:
        avg = sum(avg_vals) / len(avg_vals)
        runs_str = ", ".join(runs_formatted)
        print(f"  {name: <30} | Runs: [{runs_str}] {unit} | Avg: {avg:.2f} {unit}")

def main():
    print("=== RustFiber Benchmark Visualizer ===")
    
    # Filter out flags
    args = [arg for arg in sys.argv if not arg.startswith('--')]
    use_tracing = "--trace" in sys.argv
    
    # 1. Get Strategy
    strategy = args[1] if len(args) > 1 else "linear"
    
    # 2. Get Max Threads
    max_threads = int(args[2]) if len(args) > 2 else psutil.cpu_count(logical=True)
    
    # 3. Generate Sweep
    sweep_counts = get_sweep_counts(max_threads)
    
    print(f" Execution Sweep (Strategy={strategy}): {sweep_counts}")
    if use_tracing:
        print(f" Tracing enabled for first run of each benchmark.")

    all_results = {}
    sweep_mode = 'scaling' if len(sweep_counts) > 1 else 'single'
    os.environ['BENCH_MODE'] = sweep_mode

    for tc in sweep_counts:
        print(f" Running with {tc} Threads...")
        # Only trace the first core count to avoid massive trace files
        trace_this_run = use_tracing and tc == sweep_counts[0]
        
        for res in run_rust_benchmarks(strategy, tc, trace_this_run):
            all_results.setdefault(res['name'], []).append(res)
    
    print("\n" + "="*60)
    print(f" FINAL SCALING SUMMARY")
    print("="*60)

    for name, results in all_results.items():
        if not results: continue
        config = get_metric_config(name, results[0]['data_points'])
        print_runs_summary(name, results, config)
        
        if setup_matplotlib:
            # We use a consistent prefix based on whether it was a single run or a scaling sweep
            fname = f"{sweep_mode}_{sanitize_filename(name)}.png"
            plot_graph(name, results, fname)
            
    if use_tracing:
            print("\n" + "="*60)
            print(" GANTT CHART (CHROME TRACING) GENERATED")
            print("="*60)
            print(f" File: {os.path.abspath('trace.json')}")
            print(" How to view:")
            print(" 1. Open Chrome and navigate to chrome://tracing")
            print(" 2. OR open https://ui.perfetto.dev")
            print(" 3. Drag and drop 'trace.json' into the browser.")
            print("="*60 + "\n")

if __name__ == "__main__":
    main()
