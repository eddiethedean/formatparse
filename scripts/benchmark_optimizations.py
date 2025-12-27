#!/usr/bin/env python3
"""
Comprehensive benchmark script showcasing formatparse Rust optimizations.

This benchmark focuses on scenarios where our optimizations shine:
1. Pattern caching (LRU cache for compiled patterns)
2. Regex pre-compilation (pre-compiled search variants)
3. Fast type conversion paths
4. Pre-allocation optimizations
5. Reduced Python GIL overhead
"""

import time
import statistics
from parse import parse as parse_original  # type: ignore[import-untyped]
from formatparse import (
    parse as parse_formatparse,
    search as search_formatparse,
    findall as findall_formatparse,
)
from parse import search as search_original, findall as findall_original


def benchmark_function(func, *args, iterations, warmup=1000, **kwargs):
    """Benchmark a function with given arguments."""
    # Warmup
    for _ in range(warmup):
        func(*args, **kwargs)

    # Actual benchmark
    times = []
    for _ in range(iterations):
        start = time.perf_counter()
        func(*args, **kwargs)
        end = time.perf_counter()
        times.append((end - start) * 1000)  # Convert to milliseconds

    return {
        "mean": statistics.mean(times),
        "median": statistics.median(times),
        "total": sum(times),
    }


def print_comparison(name, original_stats, formatparse_stats):
    """Print comparison results."""
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


def test_pattern_caching():
    """Test pattern caching optimization - same pattern used repeatedly."""
    print("\n" + "=" * 70)
    print("OPTIMIZATION TEST: Pattern Caching (LRU Cache)")
    print("=" * 70)
    print("Using the same pattern repeatedly should benefit from cached FormatParser")
    print("=" * 70)

    pattern = "{name}: {age:d} years old, from {city}"
    string = "Alice: 30 years old, from NYC"
    iterations = 100000

    print(f"\nPattern: {pattern}")
    print(f"String: {string}")
    print(f"Iterations: {iterations:,}")
    print("-" * 70)

    print("Benchmarking original parse library...")
    original_stats = benchmark_function(
        parse_original, pattern, string, iterations=iterations
    )
    print(f"  Mean:   {original_stats['mean']:.4f} ms")
    print(f"  Total:  {original_stats['total']:.2f} ms")

    print("Benchmarking formatparse (with caching)...")
    formatparse_stats = benchmark_function(
        parse_formatparse, pattern, string, iterations=iterations
    )
    print(f"  Mean:   {formatparse_stats['mean']:.4f} ms")
    print(f"  Total:  {formatparse_stats['total']:.2f} ms")

    print_comparison("Pattern Caching", original_stats, formatparse_stats)


def test_many_fields():
    """Test pre-allocation optimization with many fields."""
    print("\n" + "=" * 70)
    print("OPTIMIZATION TEST: Pre-allocation (Many Fields)")
    print("=" * 70)
    print("Patterns with many fields benefit from pre-allocated vectors/HashMaps")
    print("=" * 70)

    pattern = "{f1}:{f2}:{f3}:{f4}:{f5}:{f6}:{f7}:{f8}:{f9}:{f10}:{f11}:{f12}:{f13}:{f14}:{f15}"
    string = "a:b:c:d:e:f:g:h:i:j:k:l:m:n:o"
    iterations = 50000

    print(f"\nPattern: {pattern[:60]}...")
    print("Fields: 15 fields")
    print(f"Iterations: {iterations:,}")
    print("-" * 70)

    print("Benchmarking original parse library...")
    original_stats = benchmark_function(
        parse_original, pattern, string, iterations=iterations
    )
    print(f"  Mean:   {original_stats['mean']:.4f} ms")
    print(f"  Total:  {original_stats['total']:.2f} ms")

    print("Benchmarking formatparse (with pre-allocation)...")
    formatparse_stats = benchmark_function(
        parse_formatparse, pattern, string, iterations=iterations
    )
    print(f"  Mean:   {formatparse_stats['mean']:.4f} ms")
    print(f"  Total:  {formatparse_stats['total']:.2f} ms")

    print_comparison("Many Fields", original_stats, formatparse_stats)


def test_type_conversions():
    """Test fast type conversion paths."""
    print("\n" + "=" * 70)
    print("OPTIMIZATION TEST: Fast Type Conversion Paths")
    print("=" * 70)
    print("Patterns with many type conversions benefit from fast paths")
    print("=" * 70)

    pattern = (
        "ID:{id:d} Price:{price:f} Active:{active:b} Count:{count:d} Score:{score:f}"
    )
    string = "ID:12345 Price:99.99 Active:True Count:42 Score:87.5"
    iterations = 50000

    print(f"\nPattern: {pattern}")
    print(f"String: {string}")
    print("Types: integers, floats, booleans")
    print(f"Iterations: {iterations:,}")
    print("-" * 70)

    print("Benchmarking original parse library...")
    original_stats = benchmark_function(
        parse_original, pattern, string, iterations=iterations
    )
    print(f"  Mean:   {original_stats['mean']:.4f} ms")
    print(f"  Total:  {original_stats['total']:.2f} ms")

    print("Benchmarking formatparse (with fast conversion paths)...")
    formatparse_stats = benchmark_function(
        parse_formatparse, pattern, string, iterations=iterations
    )
    print(f"  Mean:   {formatparse_stats['mean']:.4f} ms")
    print(f"  Total:  {formatparse_stats['total']:.2f} ms")

    print_comparison("Type Conversions", original_stats, formatparse_stats)


def test_search_optimization():
    """Test pre-compiled search regex optimization."""
    print("\n" + "=" * 70)
    print("OPTIMIZATION TEST: Pre-compiled Search Regex")
    print("=" * 70)
    print("search() uses pre-compiled regex variants (no anchor compilation overhead)")
    print("=" * 70)

    pattern = "Result: {value:d}"
    long_string = "A" * 5000 + "Result: 42" + "B" * 5000
    iterations = 10000

    print(f"\nPattern: {pattern}")
    print(f"String length: {len(long_string)} characters")
    print("Match position: near middle")
    print(f"Iterations: {iterations:,}")
    print("-" * 70)

    print("Benchmarking original parse library (search)...")
    original_stats = benchmark_function(
        search_original, pattern, long_string, iterations=iterations
    )
    print(f"  Mean:   {original_stats['mean']:.4f} ms")
    print(f"  Total:  {original_stats['total']:.2f} ms")

    print("Benchmarking formatparse (with pre-compiled search regex)...")
    formatparse_stats = benchmark_function(
        search_formatparse, pattern, long_string, iterations=iterations
    )
    print(f"  Mean:   {formatparse_stats['mean']:.4f} ms")
    print(f"  Total:  {formatparse_stats['total']:.2f} ms")

    print_comparison("Search Optimization", original_stats, formatparse_stats)


def test_findall_optimization():
    """Test findall with multiple matches."""
    print("\n" + "=" * 70)
    print("OPTIMIZATION TEST: Findall (Multiple Matches)")
    print("=" * 70)
    print("findall() benefits from cached patterns and optimized search")
    print("=" * 70)

    pattern = "ID:{id:d}"
    string = "ID:1 ID:2 ID:3 ID:4 ID:5 ID:6 ID:7 ID:8 ID:9 ID:10"
    iterations = 20000

    print(f"\nPattern: {pattern}")
    print(f"String: {string}")
    print("Matches: 10 matches per string")
    print(f"Iterations: {iterations:,}")
    print("-" * 70)

    print("Benchmarking original parse library (findall)...")

    # Include iteration to measure full performance (real-world usage)
    def findall_orig_with_iter(p, s):
        results = findall_original(p, s)
        _ = [r.named for r in results]  # Access all items
        return results

    original_stats = benchmark_function(
        findall_orig_with_iter, pattern, string, iterations=iterations
    )
    print(f"  Mean:   {original_stats['mean']:.4f} ms")
    print(f"  Total:  {original_stats['total']:.2f} ms")

    print("Benchmarking formatparse (findall with optimizations)...")

    # Include iteration to measure full performance (real-world usage)
    def findall_fp_with_iter(p, s):
        results = findall_formatparse(p, s)
        _ = [r.named for r in results]  # Access all items
        return results

    formatparse_stats = benchmark_function(
        findall_fp_with_iter, pattern, string, iterations=iterations
    )
    print(f"  Mean:   {formatparse_stats['mean']:.4f} ms")
    print(f"  Total:  {formatparse_stats['total']:.2f} ms")

    print_comparison("Findall", original_stats, formatparse_stats)


def test_case_insensitive():
    """Test case-insensitive matching with pre-compiled regex."""
    print("\n" + "=" * 70)
    print("OPTIMIZATION TEST: Case-Insensitive Matching")
    print("=" * 70)
    print("Case-insensitive search uses pre-compiled regex (no runtime compilation)")
    print("=" * 70)

    pattern = "Name: {name}"
    string = "Name: ALICE"
    iterations = 50000

    print(f"\nPattern: {pattern}")
    print(f"String: {string} (uppercase)")
    print("Case sensitive: False")
    print(f"Iterations: {iterations:,}")
    print("-" * 70)

    print("Benchmarking original parse library (case_sensitive=False)...")
    original_stats = benchmark_function(
        parse_original, pattern, string, case_sensitive=False, iterations=iterations
    )
    print(f"  Mean:   {original_stats['mean']:.4f} ms")
    print(f"  Total:  {original_stats['total']:.2f} ms")

    print("Benchmarking formatparse (with pre-compiled case-insensitive regex)...")
    formatparse_stats = benchmark_function(
        parse_formatparse, pattern, string, case_sensitive=False, iterations=iterations
    )
    print(f"  Mean:   {formatparse_stats['mean']:.4f} ms")
    print(f"  Total:  {formatparse_stats['total']:.2f} ms")

    print_comparison("Case-Insensitive", original_stats, formatparse_stats)


def test_cache_warmup():
    """Test cache warmup effect - first call vs subsequent calls."""
    print("\n" + "=" * 70)
    print("OPTIMIZATION TEST: Cache Warmup Effect")
    print("=" * 70)
    print("First call compiles pattern, subsequent calls use cache")
    print("=" * 70)

    pattern = "{user}:{action}:{target}"
    string = "alice:login:server1"
    iterations = 1000

    print(f"\nPattern: {pattern}")
    print(f"String: {string}")
    print("Testing: First 10 calls (cold) vs next 990 calls (warm)")
    print("-" * 70)

    # Cold cache - first few calls
    print("Benchmarking formatparse (cold cache - first 10 calls)...")
    cold_times = []
    for i in range(10):
        start = time.perf_counter()
        parse_formatparse(pattern, string)
        end = time.perf_counter()
        cold_times.append((end - start) * 1000)
    cold_mean = statistics.mean(cold_times)
    print(f"  Mean:   {cold_mean:.4f} ms")
    print(f"  First call: {cold_times[0]:.4f} ms")

    # Warm cache - subsequent calls
    print("\nBenchmarking formatparse (warm cache - next 990 calls)...")
    warm_times = []
    for i in range(iterations - 10):
        start = time.perf_counter()
        parse_formatparse(pattern, string)
        end = time.perf_counter()
        warm_times.append((end - start) * 1000)
    warm_mean = statistics.mean(warm_times)
    print(f"  Mean:   {warm_mean:.4f} ms")

    speedup = cold_mean / warm_mean
    print(f"\n{'=' * 70}")
    print(f"✅ Cache provides {speedup:.2f}x speedup after warmup")
    print(f"{'=' * 70}")


def test_mixed_patterns():
    """Test cache with many different patterns."""
    print("\n" + "=" * 70)
    print("OPTIMIZATION TEST: Mixed Patterns (Cache Management)")
    print("=" * 70)
    print("Testing cache behavior with many different patterns (LRU eviction)")
    print("=" * 70)

    patterns = [
        "{name}:{age:d}",
        "{id:d}:{status}",
        "{x:f},{y:f}",
        "{a}:{b}:{c}",
        "{num:d}",
    ]
    strings = [
        "Alice:30",
        "123:active",
        "1.5,2.5",
        "a:b:c",
        "42",
    ]
    iterations = 20000

    print(f"\nPatterns: {len(patterns)} different patterns")
    print(f"Iterations per pattern: {iterations // len(patterns):,}")
    print(f"Total iterations: {iterations:,}")
    print("-" * 70)

    # Original parse
    print("Benchmarking original parse library (mixed patterns)...")
    original_times = []
    for pattern, string in zip(patterns, strings):
        for _ in range(iterations // len(patterns)):
            start = time.perf_counter()
            parse_original(pattern, string)
            end = time.perf_counter()
            original_times.append((end - start) * 1000)
    original_mean = statistics.mean(original_times)
    print(f"  Mean:   {original_mean:.4f} ms")
    print(f"  Total:  {sum(original_times):.2f} ms")

    # Formatparse
    print("Benchmarking formatparse (with LRU cache, mixed patterns)...")
    formatparse_times = []
    for pattern, string in zip(patterns, strings):
        for _ in range(iterations // len(patterns)):
            start = time.perf_counter()
            parse_formatparse(pattern, string)
            end = time.perf_counter()
            formatparse_times.append((end - start) * 1000)
    formatparse_mean = statistics.mean(formatparse_times)
    print(f"  Mean:   {formatparse_mean:.4f} ms")
    print(f"  Total:  {sum(formatparse_times):.2f} ms")

    original_stats = {"mean": original_mean, "total": sum(original_times)}
    formatparse_stats = {"mean": formatparse_mean, "total": sum(formatparse_times)}
    print_comparison("Mixed Patterns", original_stats, formatparse_stats)


def test_long_string_search():
    """Test search in very long strings."""
    print("\n" + "=" * 70)
    print("OPTIMIZATION TEST: Long String Search")
    print("=" * 70)
    print("Searching in very long strings benefits from optimized search regex")
    print("=" * 70)

    pattern = "Found: {value:d}"
    # Create a very long string with match at the end
    long_string = "X" * 50000 + "Found: 999"
    iterations = 1000

    print(f"\nPattern: {pattern}")
    print(f"String length: {len(long_string):,} characters")
    print("Match position: at end")
    print(f"Iterations: {iterations:,}")
    print("-" * 70)

    print("Benchmarking original parse library (search)...")
    original_stats = benchmark_function(
        search_original, pattern, long_string, iterations=iterations
    )
    print(f"  Mean:   {original_stats['mean']:.4f} ms")
    print(f"  Total:  {original_stats['total']:.2f} ms")

    print("Benchmarking formatparse (optimized search)...")
    formatparse_stats = benchmark_function(
        search_formatparse, pattern, long_string, iterations=iterations
    )
    print(f"  Mean:   {formatparse_stats['mean']:.4f} ms")
    print(f"  Total:  {formatparse_stats['total']:.2f} ms")

    print_comparison("Long String Search", original_stats, formatparse_stats)


def main():
    """Run all optimization benchmarks."""
    print("=" * 70)
    print("formatparse Rust Optimizations Benchmark Suite")
    print("=" * 70)
    print("\nThis benchmark suite focuses on scenarios where our Rust")
    print("optimizations provide the most benefit:")
    print("  • Pattern caching (LRU cache)")
    print("  • Regex pre-compilation")
    print("  • Fast type conversion paths")
    print("  • Pre-allocation optimizations")
    print("  • Reduced Python GIL overhead")
    print("=" * 70)

    # Run all optimization tests
    test_pattern_caching()
    test_many_fields()
    test_type_conversions()
    test_search_optimization()
    test_findall_optimization()
    test_case_insensitive()
    test_cache_warmup()
    test_mixed_patterns()
    test_long_string_search()

    print("\n" + "=" * 70)
    print("Optimization Benchmark Suite Complete!")
    print("=" * 70)


if __name__ == "__main__":
    main()
