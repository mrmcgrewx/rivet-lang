# Independent median and benchmark review

Reviewed on 2026-10-25 against `MEDIAN-PLAN.md` and implementation checkpoint `b72a8bd569f217ce9ce31358c261fe0e099b94f5`. The reviewer implemented the UI, but did not implement the binary algorithm, compiler extension, C runtime change, speed profile or benchmark reviewed here. This review changed only this document. Independent probes used temporary files outside the repository; no Git mutations were made.

## Result

No unresolved correctness or acceptance blocker was found. The median algorithm remains in the 803-byte, 81-operation binary source and its generated native control flow. The production C runtime provides generic input, array storage and finite-float output, with no median solver. The benchmark retains generated operations and runtime checks and distinguishes whole-process cost from repeated preloaded search.

## Production review

- The decoder checks exact payloads, register types and finite literal bits. Float copy preserves bits, including negative zero. New operations participate in read/destination tracking, and float return terminates control flow. The reference implementation and generated code both reject either signed zero divisor and nonfinite arithmetic results. LLVM loads operands before storing the destination, including aliased registers; integer conversion uses `sitofp`, arithmetic uses ordinary `fadd`/`fdiv`, and finite-result guards precede storage. No fast-math flags weaken these checks.
- The source swaps array handles and lengths before partitioning, so `m <= n`. For every searched `i` in `[0,m]`, `j = (m+n+1)/2-i` is in `[0,n]`. Guarded endpoint reads use initialized sentinels; a failed cross-edge comparison strictly reduces the search interval. Odd totals select the maximum left edge. Even totals convert both middle integers before addition, avoiding integer overflow. For the stated integer bounds, every median is exactly representable as binary64. Search uses logarithmic iterations and constant auxiliary state; input loading still uses linear time/storage. Sortedness and element bounds remain documented caller preconditions.
- Native float output checks finiteness and remaining input before writing. Its fixed buffer accommodates all finite binary64 values in the specified `%.16e` format. The reference formatter normalizes exponent sign/width to match. The speed profile uses `-O3` and dead stripping without LTO or fast-math; baseline/size settings and existing operations retain their meaning.

## Independent checks performed

| Probe | Result |
| --- | --- |
| Exhaustive small-input oracle | **63,503 sorted-array pairs** agreed with a separate sorted-union oracle: every pair of nondecreasing arrays of length 0–5 over values -2 through 2, excluding two empty arrays. Reference result bits matched the oracle. |
| Independent native differential cases | **72 boundary/seeded cases**, including lengths 0, 1, 2, 31, 127 and 1000, duplicate-heavy and full-range values, empty sides and argument-order changes. Reused one executable per profile: **216 native launches** across baseline, size and speed matched reference output and oracle bits. Seed: `0xd1ffe2e7a12026`. |
| Float encoding/formatting | **10,000 finite binary64 patterns**, including signed zero, subnormal/normal boundaries and finite extrema, roundtripped source bytes and output bits. Every reference output string matched the host's actual C `snprintf("%.16e\n", value)` through FFI. **Five nonfinite encodings** were rejected. Seed: `0x466c6f61744d6564`. This was a formatter/libc comparison, not 10,000 native artifact builds. |
| Independent benchmark smoke | Actual runner with seed **20261103**, three cases per family, two samples, two warmup rounds and a 1 ms target: **24 corpus cases**, **72 unchanged standalone validations**, **192 raw observations**. All checksums, result checks and harness boundaries passed. Only the temporary artifact destination was redirected outside the repository. The report classified this as `functional_smoke`; its timings are not performance evidence. |
| Smoke report and emitted-code audit | Checked per-child macOS RSS raw values equaled normalized bytes; measured call counts, warmups, weighted checksums, randomized job sets and two-sample distributions agreed. Reversing the four LLVM symbol substitutions restored the original bytes exactly, with **81 static tick sites** retained and **57–322 executed ticks** across this corpus. C disassembly retained calls through the measured harness to the separately compiled partition function. |

## Benchmark audit

The C and Python baselines use the same shorter-array partition algorithm; only the independent oracle sorts. C's solver and outer loop are separate translation units with no LTO, preventing body-based call hoisting. Printed weighted checksums consume every result, and individual oracle checks precede timing. Their parser and safety-check differences are explicitly disclosed.

The Rivet harness substitutes only entry, input, float-output and cleanup boundaries. It supplies real runtime array handles, resets fuel between invocations, verifies boundary counts and unchanged allocation charge, and performs actual cleanup after timing. Generated branches, arithmetic, array checks and fuel calls remain. Additional harness bookkeeping is included in Rivet's measured time. Control loops are reported as floors and never subtracted.

The runner measures each exact child with `wait4(pid, ...)`, preserving macOS byte units and converting Linux KiB separately. Process timing includes launch through exit; preloaded timing excludes parsing/setup/formatting while retaining calls, loops and checksums. Warmups are recorded but excluded from summaries; job order is randomized and calibration/call counts are retained. Whole-child peak RSS is correctly distinguished from auxiliary search storage and from the different preloaded case-pool memory scope.

The coordinator's full measurement is recorded separately in [results](MEDIAN-BENCHMARK-RESULTS.md) and the [raw report](MEDIAN-BENCHMARK.json). All reviewer probes had stopped before its approximately 21:05–21:06 UTC measurement window; the independent smoke completed at 21:02:42 UTC and the last formatter probe completed after its 21:03:08 UTC build. No background reviewer test process remained. The full measurement was not independently rerun here.

These bounded probes are not a formal proof. Linux RSS handling was inspected but not executed on this macOS host. The comparison covers this algorithm, warmed case pools and these implementations; it establishes neither general language superiority nor an isolated hardware benchmark. Full regression, actual in-app acceptance, clean-clone verification and public CI are coordinator-owned evidence in [validation](MEDIAN-VALIDATION.md), not additional independent runs claimed by this review. Temporary reviewer probes and artifacts were removed after recording these results.
