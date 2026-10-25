# Median performance results

Measured on 2026-10-25 from clean public revision `b72a8bd569f217ce9ce31358c261fe0e099b94f5` on **Apple M1 Pro, macOS 26.5.2 arm64**, using Apple Clang 17.0.0 and CPython 3.11.9. Rivet and C use -O3; Rivet retains its runtime safety checks. The [method](MEDIAN-BENCHMARK-METHOD.md) and [complete raw report](MEDIAN-BENCHMARK.json) make this comparison reproducible.

The full run verified **128 input pairs** against an independent sorted-union oracle and all three unchanged standalone executables. Eight families contain sixteen cases each; seven measured samples follow warmup. All checksums and harness boundary checks passed. The report contains 552 raw observations, including calibration, controls and discarded warmups. These are measurements of one algorithm and these implementations, not a general language ranking.

## Repeated preloaded search

Times below are median **nanoseconds per query** for each family. Input parsing, setup and formatting are outside the timed region. Function calls, loop/checksum overhead, Rivet fuel/type/bounds/arithmetic checks and harness boundary bookkeeping remain inside. The same algorithm and case distribution are used in all languages. Each sample cycles over a fixed warmed pool; this does not represent cold-cache application workloads.

| Input family | Rivet (ns) | C (ns) | Python (ns) |
| --- | ---: | ---: | ---: |
| Tiny / published | 184.10 | 5.11 | 587.94 |
| 1000 + 1000 random | 558.91 | 34.19 | 1439.22 |
| Asymmetric | 194.24 | 5.01 | 622.42 |
| One empty side | 134.47 | 2.99 | 486.47 |
| Disjoint ranges | 488.45 | 17.20 | 1210.05 |
| Duplicate-heavy | 136.78 | 3.23 | 514.25 |
| Interleaved | 140.74 | 3.73 | 528.65 |
| Negative values | 491.05 | 17.09 | 1233.40 |

For two 1000-element random arrays, Rivet is **2.58× faster than Python** in this harness, while C is **16.35× faster than Rivet**. This is useful evidence of remaining native overhead; it does not establish a measured cause for the whole gap. Rivet executes 57–324 fuel-counted source instructions across this corpus and retains checked runtime array accesses. C has no equivalent instruction budget/tagged-value checks; Python retains its interpreter and object semantics.

For that family, the median control-loop floors are rivet: 1.24 ns, c: 1.26 ns, python: 70.16 ns. Controls are reported separately and never subtracted. Per-query p10–p90 sample ranges are rivet: 558.11–561.36 ns, c: 33.85–34.46 ns, python: 1438.27–1446.73 ns. These ranges describe the seven samples, not confidence intervals.

## Whole-process latency and memory

Each observation below launches a fresh executable/interpreter for one fixed representative input pair in that family. Elapsed time includes launch, input parsing, search, formatting and exit. Peak RSS covers the entire child process, including the interpreter/runtime and temporary parsing allocations; **it is not auxiliary algorithm memory**. Each row reports medians of seven samples. MiB means 1,048,576 bytes.

| Input family | Rivet ms / MiB | C ms / MiB | Python ms / MiB |
| --- | ---: | ---: | ---: |
| Tiny / published | 2.097 / 1.359 | 2.167 / 1.328 | 24.322 / 14.969 |
| 1000 + 1000 random | 1.960 / 1.391 | 2.227 / 1.328 | 24.458 / 15.125 |
| Asymmetric | 2.179 / 1.375 | 1.953 / 1.328 | 24.344 / 14.891 |
| One empty side | 2.093 / 1.375 | 2.314 / 1.328 | 24.431 / 14.906 |
| Disjoint ranges | 2.015 / 1.375 | 2.180 / 1.328 | 24.578 / 14.953 |
| Duplicate-heavy | 2.264 / 1.375 | 2.162 / 1.328 | 24.719 / 15.000 |
| Interleaved | 1.947 / 1.391 | 1.898 / 1.328 | 24.773 / 15.047 |
| Negative values | 2.048 / 1.375 | 2.402 / 1.328 | 24.714 / 14.953 |

Native launch-to-exit medians are around 2 ms, while preloaded searches take fractions of a microsecond. Startup and input/output therefore dominate these tiny standalone workloads; the small C/Rivet process-time differences should not be interpreted as their search ranking. Python process latency includes interpreter startup.

Preloaded-process RSS is a different scope: those children hold all sixteen cases in a family. For balanced 1000 + 1000 arrays, median whole-child peak RSS during the preloaded run was rivet: 1.625 MiB, c: 1.594 MiB, python: 18.844 MiB. All three implementations use O(1) auxiliary search state; storing supplied inputs is O(m+n).

Standalone native file sizes were **34,792 bytes for Rivet** and **33,848 bytes for C**. Python script size would omit the interpreter and is not an equivalent executable-size comparison. Compilation time is outside runtime measurements.

## Reproduce and limits

```sh
cargo build --release --locked
python3 scripts/median_benchmark.py --report .rivet/median-benchmark.json
```

The published run used defaults: seed 20261025, seven samples, sixteen cases per family, thirty-two warmup rounds and a 50 ms target with a two-million-call cap. Short native cases can hit that cap before 50 ms; actual durations/call counts are in the report. Job order was randomized. Our build/test suites were stopped during measurement; the desktop, workbench and ordinary background services remained active. No CPU pinning, frequency control, energy or allocation-count measurement is claimed.

Correctness smoke runs in CI exercise the same harness with fewer cases/samples and are explicitly marked functional_smoke. They are not used as performance evidence. This benchmark does not measure AI authoring quality, token use, Java/JavaScript runtimes, other algorithms or general application performance. Future comparisons should preserve this separation of correctness, process cost, preloaded computation and memory scope.
