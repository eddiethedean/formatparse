#!/usr/bin/env python3
"""
Benchmark script to compare formatparse vs parse library performance.
"""

import time
import statistics
from parse import parse as parse_original  # type: ignore[import-untyped]
from formatparse import parse as parse_formatparse

# Test cases
TEST_CASES = [
    {
        "name": "Simple named fields",
        "pattern": "{name}: {age:d}",
        "string": "Alice: 30",
        "iterations": 100000,
    },
    {
        "name": "Multiple named fields",
        "pattern": "{name} is {age:d} years old and lives in {city}",
        "string": "Alice is 30 years old and lives in NYC",
        "iterations": 50000,
    },
    {
        "name": "Positional fields",
        "pattern": "{}, {}",
        "string": "Hello, World",
        "iterations": 100000,
    },
    {
        "name": "Complex pattern with types",
        "pattern": "Value: {value:f}, Count: {count:d}, Active: {active:b}",
        "string": "Value: 3.14159, Count: 42, Active: True",
        "iterations": 50000,
    },
    {
        "name": "No match (should fail fast)",
        "pattern": "{name}: {age:d}",
        "string": "This doesn't match at all",
        "iterations": 100000,
    },
    {
        "name": "Long string with match at end",
        "pattern": "Result: {result:d}",
        "string": "A" * 1000 + "Result: 42",
        "iterations": 10000,
    },
]


def benchmark_function(func, pattern, string, iterations, warmup=1000):
    """Benchmark a parsing function."""
    # Warmup
    for _ in range(warmup):
        func(pattern, string)

    # Actual benchmark
    times = []
    for _ in range(iterations):
        start = time.perf_counter()
        func(pattern, string)
        end = time.perf_counter()
        times.append((end - start) * 1000)  # Convert to milliseconds

    return {
        "mean": statistics.mean(times),
        "median": statistics.median(times),
        "stdev": statistics.stdev(times) if len(times) > 1 else 0,
        "min": min(times),
        "max": max(times),
        "total": sum(times),
    }


def run_benchmark(test_case):
    """Run benchmark for a single test case."""
    name = test_case["name"]
    pattern = test_case["pattern"]
    string = test_case["string"]
    iterations = test_case["iterations"]

    print(f"\n{'=' * 70}")
    print(f"Test: {name}")
    print(f"Pattern: {pattern}")
    print(f"String: {string[:50]}{'...' if len(string) > 50 else ''}")
    print(f"Iterations: {iterations:,}")
    print(f"{'-' * 70}")

    # Benchmark original parse
    print("Benchmarking original parse library...")
    try:
        original_stats = benchmark_function(parse_original, pattern, string, iterations)
        print(f"  Mean:   {original_stats['mean']:.4f} ms")
        print(f"  Median: {original_stats['median']:.4f} ms")
        print(f"  Total:  {original_stats['total']:.2f} ms")
    except Exception as e:
        print(f"  ERROR: {e}")
        original_stats = None

    # Benchmark formatparse
    print("Benchmarking formatparse...")
    try:
        formatparse_stats = benchmark_function(
            parse_formatparse, pattern, string, iterations
        )
        print(f"  Mean:   {formatparse_stats['mean']:.4f} ms")
        print(f"  Median: {formatparse_stats['median']:.4f} ms")
        print(f"  Total:  {formatparse_stats['total']:.2f} ms")
    except Exception as e:
        print(f"  ERROR: {e}")
        formatparse_stats = None

    # Compare results
    if original_stats and formatparse_stats:
        speedup = original_stats["mean"] / formatparse_stats["mean"]
        print(f"\n{'=' * 70}")
        if speedup > 1:
            print(f"✅ formatparse is {speedup:.2f}x FASTER")
        elif speedup < 1:
            print(f"⚠️  formatparse is {1 / speedup:.2f}x SLOWER")
        else:
            print("⚖️  formatparse is about the same speed")
        print(f"{'=' * 70}")


def main():
    """Run all benchmarks."""
    print("=" * 70)
    print("formatparse vs parse Library Benchmark")
    print("=" * 70)

    for test_case in TEST_CASES:
        run_benchmark(test_case)

    print("\n" + "=" * 70)
    print("Benchmark complete!")
    print("=" * 70)


if __name__ == "__main__":
    main()
