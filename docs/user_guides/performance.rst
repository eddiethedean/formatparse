Performance
===========

formatparse is designed to be **fast** on hot paths: pattern compilation is cached, type
conversion is optimized in Rust, and batch APIs reduce repeated work.

Benchmarks depend on pattern complexity, field types, and input size. For reproducible
numbers on your machine, use the scripts in the repository:

- ``scripts/benchmark.py`` — general comparisons.
- ``scripts/benchmark_optimizations.py`` and ``scripts/compare_benchmarks.py`` — optimization-focused runs.

Treat headline speedup figures in marketing material as **order-of-magnitude** guides, not
guarantees for every workload.

When profiling your own app:

- Prefer :func:`~formatparse.compile` (or :func:`~formatparse.parse` which shares the same
  cache) when the same pattern is used many times.
- Use :func:`~formatparse.parse_batch` when parsing many strings with one compiled parser.
- Use :func:`~formatparse.findall_iter` when you want incremental matches without building a
  full list up front.
