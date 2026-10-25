# Native fuel optimization: measured results

On the Apple M1 Pro, the unchanged balanced 1000 + 1000 median workload improved from **559.45 to 104.14 ns/query**, a **5.37× speedup**. Fresh C measurements were 34.64 ns before and 34.47 ns after: C’s advantage narrowed from **16.15× to 3.02×**. The original historical result was 16.35×. This is a measured improvement for this workload and host, with Rivet’s safety semantics retained; it is not language-wide performance parity.

## Why it was slow and what changed

The original generated code called `rv2_tick` before every source instruction. In the balanced corpus, that meant 67–322 runtime calls per query, plus checked array access. A bounded CPU sample put **54.9%** of samples inside the tick helper and **14.7%** inside array access; call barriers and register preservation also affected the surrounding code. These percentages are sampled locations, not a precise decomposition of total overhead.

The compiler now exposes the exact fuel check to LLVM through an `available_externally`/`alwaysinline` body and a stable counter-address getter. The backend can inline it, reuse the counter address and simplify equivalent machine operations. The inspected speed-profile evaluation function goes from **85 static tick-call sites to zero**, with one getter call; backend block duplication explains why the old native call-site count differs from the 81 source instructions. This applies to all RVB2 computations, with performance measured here only for median.

Every emitted source tick remains, the limit stays at one million, and the check still precedes the operation. Arithmetic overflow, Float validity/division, type/index bounds, input, allocation and cleanup behavior are preserved. No `.rvb` source, algorithm, C/Python baseline, benchmark harness, runner, compiler profile or optimization flag changed. No LTO, fast-math or median-specific runtime path was added. The source language cannot access the counter getter. See [validation](NATIVE-OPTIMIZATION-VALIDATION.md), [independent review](NATIVE-OPTIMIZATION-REVIEW.md) and [sampling/disassembly evidence](evidence/native-optimization-profile.json).

## Same benchmark, fresh controls

Both runs used clean checkouts: before `b72a8bd569f217ce9ce31358c261fe0e099b94f5` at 2026-10-25T21:31:49Z; after `c279276de3866f1e918fc8394900abde62208b1c` at 2026-10-25T21:32:11Z. They ran sequentially after our builds, tests, reviewers and profilers stopped. Host: macOS 26.5.2 arm64, Apple M1 Pro, Apple Clang 17.0.0, CPython 3.11.9. Ordinary background desktop activity was not controlled; the fresh C/Python controls help assess drift.

The [existing method](MEDIAN-BENCHMARK-METHOD.md) and defaults are unchanged: seed 20261025, eight families × 16 input pairs, seven samples, 32 warmup rounds, 50 ms target and two-million-call cap. Each run verified all 128 pairs against the independent sorted-union oracle and all three standalone programs, then verified the preloaded results/checksums. Each report contains 552 raw observations. Corpus, parameters, randomized orders, source/baseline hashes, all 81 source tick sites and per-case executed-tick vectors match the historical benchmark. [Machine-checked comparison](evidence/native-optimization-comparison.json).

**Preloaded evaluation medians, ns/query; lower is better.** Includes safety, boundary handling and checksum/loop overhead; excludes parsing, formatting and startup. No harness control is subtracted.

| Input family | Rivet before | Rivet after | Speedup | C after | Rivet / C | Python after |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| tiny | 184.20 | 33.58 | 5.49× | 5.15 | 6.51× | 588.43 |
| balanced_1000 | 559.45 | 104.14 | 5.37× | 34.47 | 3.02× | 1443.81 |
| asymmetric | 194.75 | 35.94 | 5.42× | 5.00 | 7.19× | 626.00 |
| empty_side | 134.95 | 24.75 | 5.45× | 3.04 | 8.13× | 489.95 |
| disjoint | 488.62 | 94.39 | 5.18× | 17.42 | 5.42× | 1206.12 |
| duplicates | 136.78 | 27.32 | 5.01× | 3.24 | 8.44× | 519.28 |
| interleaved | 140.58 | 30.81 | 4.56× | 3.76 | 8.19× | 530.23 |
| negative | 492.05 | 88.28 | 5.57× | 17.31 | 5.10× | 1249.79 |

All eight families improved by 4.56×–5.57×. Across the reruns, C medians changed by -0.51% to +1.33% and Python by -0.50% to +1.00%. These small control shifts do not explain Rivet’s improvement.

Balanced-family p10–p90 was **557.26–593.95 ns before** and **103.50–104.95 ns after**. After-run C was 33.97–34.67 ns and Python 1435.60–1451.02 ns. These are seven-sample percentile ranges, not confidence intervals. The original run’s 558.91 ns Rivet median is close to the fresh 559.45 ns baseline. After-run harness-control medians were 1.27 ns Rivet, 1.26 ns C and 70.22 ns Python; they are floors, not removable-cost estimates.

## Process latency and memory

**Whole-process medians, milliseconds.** These include child launch, parsing, search, formatting and waiting. The large search speedup does not translate into a comparable launch-time improvement.

| Input family | Rivet before | Rivet after | C after | Python after |
| --- | ---: | ---: | ---: | ---: |
| tiny | 1.994 | 1.864 | 2.069 | 24.207 |
| balanced_1000 | 1.856 | 1.829 | 2.257 | 24.541 |
| asymmetric | 2.222 | 2.117 | 2.323 | 24.336 |
| empty_side | 2.260 | 2.157 | 2.212 | 24.721 |
| disjoint | 1.826 | 1.890 | 2.264 | 23.863 |
| duplicates | 2.211 | 2.210 | 1.998 | 24.654 |
| interleaved | 2.249 | 2.072 | 1.871 | 24.195 |
| negative | 1.980 | 1.976 | 2.132 | 25.200 |

**Balanced-family peak RSS medians, MiB (1 MiB = 1048576 bytes).** Each value is the peak of an entire child, including setup and warmup. The preloaded child holds sixteen input pairs, while the standalone child holds one; neither number is auxiliary search memory. All family distributions remain in the raw reports.

| Scope | Implementation | Before | After |
| --- | --- | ---: | ---: |
| process | rivet | 1.406 | 1.406 |
| process | c | 1.328 | 1.344 |
| process | python | 15.016 | 15.094 |
| preloaded | rivet | 1.625 | 1.641 |
| preloaded | c | 1.594 | 1.609 |
| preloaded | python | 18.734 | 18.875 |

Rivet’s standalone median peak stayed at **1.406 MiB**; preloaded peak increased by one **16 KiB** page, from 1.625 to 1.641 MiB. C’s two scopes also increased by one page in these runs. This is no demonstrated memory reduction and is not evidence of a leak. Logical allocation behavior and limits are unchanged.

## Tradeoffs and remaining gap

The standalone speed executable grew from **34792 to 34808 bytes** (+16); the preloaded executable similarly grew from 35056 to 35072 bytes. Native code within the measured function grew from 249 to 493 ARM64 instructions as the checks were inlined; executable file size also reflects linking/alignment and is not a direct code-instruction metric.

A separate native-build comparison used the same two prebuilt release compilers, unchanged source and speed profile, one discarded warmup plus seven measured builds per version, shuffled within each round. Median compiler-reported native build time rose from **298 to 320 ms** (+22 ms, 7.4%); observed ranges were 288–306 and 309–332 ms. Outer process medians were 302.53 and 324.60 ms. All produced artifacts returned 2.5 for the sample. This measures building this small native program, not rebuilding the Rust compiler. [Build observations and commands](evidence/native-optimization-builds.json).

C remains **3.02× faster** on balanced inputs and **5.10×–8.44×** faster on the other measured families. The remaining checked array calls, fuel operations, checked arithmetic and Rivet harness boundaries are costs C does not share identically. After optimization, checked array access accounts for 26.1% of the diagnostic CPU sample; the remaining generated-code samples include both search and inlined checks. Further work should make generic array operations more visible to the optimizer with the same safety guarantees and measure again. No second optimization is claimed here.

## Reproduce and inspect

Use separate clean checkouts at `b72a8bd` and `c279276`; build each compiler with `cargo build --release --locked`. Run the following sequentially from the respective checkout, writing reports outside it:

```sh
python3 scripts/median_benchmark.py --report /absolute/path/before.json
python3 scripts/median_benchmark.py --report /absolute/path/after.json
```

Use the first command in the old checkout and the second in the optimized checkout. Do not mix an old compiler with the new runtime/harness directory. Keep our other test/build/profile jobs stopped. The report records exact build commands, compiler/runtime/source hashes, corpus, raw observations, host and settings.

- [Fresh before report](NATIVE-OPTIMIZATION-BEFORE.json)
- [Fresh after report](NATIVE-OPTIMIZATION-AFTER.json)
- [Historical median report, retained unchanged](MEDIAN-BENCHMARK.json)
- [Validation and public CI](NATIVE-OPTIMIZATION-VALIDATION.md)
