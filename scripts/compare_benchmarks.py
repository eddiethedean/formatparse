#!/usr/bin/env python3
"""Compare benchmark results with baseline."""

import json
import sys

try:
    with open('baseline_benchmarks.json') as f:
        baseline = json.load(f)
    with open('benchmark_results.json') as f:
        current = json.load(f)
    
    # Create dicts for easy lookup
    baseline_dict = {b['name']: b['stats']['mean'] for b in baseline['benchmarks']}
    current_dict = {c['name']: c['stats']['mean'] for c in current['benchmarks']}
    
    regressions = []
    for name, current_mean in current_dict.items():
        if name in baseline_dict:
            baseline_mean = baseline_dict[name]
            if baseline_mean > 0:
                change_pct = ((current_mean - baseline_mean) / baseline_mean) * 100
                if change_pct > 10:  # More than 10% slower
                    regressions.append((name, change_pct, baseline_mean, current_mean))
    
    if regressions:
        print('Performance regressions detected (>10% slower):')
        for name, pct, baseline, current in regressions:
            print(f'  {name}: {pct:.2f}% slower ({baseline:.6f}s -> {current:.6f}s)')
        sys.exit(1)
    else:
        print('No significant performance regressions detected.')
except FileNotFoundError:
    print('No baseline found, creating one...')
    import shutil
    shutil.copy('benchmark_results.json', 'baseline_benchmarks.json')
    print('Baseline created successfully.')

