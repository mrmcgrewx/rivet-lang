# Median comparison method

This benchmark compares the same shorter-array binary-partition median algorithm in Rivet, C and Python. It is a workload comparison with explicit process and preloaded scopes, not a language-wide speed or memory ranking. The binary Rivet source is `examples/median-two-sorted-arrays.rvb`; C/Python comparison programs and the benchmark-only harness are in `benchmarks/median/`. The coordinator's solution tests are separate from benchmark validation.

## Reproduce

On the verified macOS Apple Silicon host, with the required Rust/Clang toolchains installed:

```sh
cargo build --release --locked
python3 scripts/median_benchmark.py --report .rivet/median-benchmark.json
```

The default run uses seed 20261025, eight input families with sixteen cases each, seven measurement samples, thirty-two warmup rounds per preloaded child and a fifty-millisecond target per preloaded sample. Arguments allow controlled reruns; the report records them. `--rivet PATH` selects a compiler, including a debug compiler for correctness checks. Builds and exact input files remain in a fresh ignored `.rivet/benchmarks/median-*` directory. Existing artifacts are not overwritten.

A bounded CI or integration smoke is:

```sh
python3 scripts/median_benchmark.py --rivet target/debug/rivet \
  --samples 1 --cases-per-group 2 --target-ms 1 \
  --report .rivet/median-benchmark-smoke.json
```

Runs with fewer than five samples, less than a ten-millisecond target, or fewer than eight cases per family are marked `functional_smoke` in the report. Their numerical timings are diagnostic, not performance evidence. The complete default comparison should run while other builds/tests are idle. The runner does not pin cores, disable background services, control temperature/frequency, or claim a laboratory-isolated machine.

## Inputs and correctness

The fixed-seed corpus includes tiny/published/fractional cases, balanced 1000-element arrays, strongly asymmetric lengths, either side empty, disjoint ranges, duplicate-heavy data, interleaved arrays and negative values. Both argument orders exercise searching the shorter array. Arrays are sorted; each length is at most 1000, total length is nonzero, and elements stay within the problem's +/-1000000 precondition. Those preconditions are identical across implementations. Search uses O(log(min(m,n)+1)) iterations and O(1) auxiliary state; reading/storing the input still takes O(m+n).

Before timing, an independent Python sorted-union oracle calculates every expected median. Every case is run through all three **unchanged standalone programs**, and its exact canonical decimal output is checked. Every preloaded child also validates each individual result against the same oracle values before its clock starts. Thus aggregate checksums do not replace case-by-case validation, and the substituted Rivet harness is checked against the untouched executable on the entire corpus.

C and Python implement the same partition, endpoint sentinels, cross-edge comparison, odd/even decision and conversion-before-average algorithm as the binary source. They do not use sorting/merging as the measured solver. The oracle alone sorts. C uses ordinary guarded array accesses and length validation but has no Rivet-style instruction fuel, tagged values or checked-integer ABI; Python has its normal interpreter/object/indexing semantics. These implementation and safety-overhead differences remain in the measurements and are not hidden by dropping Rivet checks.

The standalone parsers intentionally follow their languages' straightforward conventions: C uses `scanf` over valid benchmark-domain integers, Rivet uses its bounded runtime input parser, and Python reads bounded stdin and converts tokens to integers. Their rejection behavior for arbitrary malformed/out-of-range text is not claimed equivalent. The benchmark concerns the identical valid corpus. Benchmark-only Python imports (`json`, `math`, `time`) occur inside the preloaded entry, so they do not artificially inflate the standalone Python startup comparison. Normal Python interpreter startup, allocation and default garbage-collection behavior remain enabled.

## Build policy and optimizer visibility

Rivet uses its explicit `speed` profile: `-O3` plus dead stripping, retaining runtime checks. C uses the same installed Clang, `-O3`, function/data sections and dead stripping. No fast-math or link-time optimization is enabled. `partition.c` and the C harness are separate translation units; the compiler cannot see the algorithm body while compiling the outer benchmark loop and hoist repeated calls out of it. The generated Rivet LLVM is also compiled separately from its C runtime/harness. Python uses the installed interpreter, whose implementation and version are recorded.

Outputs feed a weighted checksum that is printed and independently verified. Each call's result is multiplied by `1 + case_index % 17`; a full-round expected checksum is calculated from oracle results. All corpus medians are integers or halves, and the configured call cap keeps these sums exactly representable as binary64. The harness control uses a volatile expected-value read in C; Python uses a real function call returning the expected value. No result is an unused dead computation.

## Scope 1: whole process

Each sample starts a fresh standalone child for one fixed representative case per input family. The parent monotonic clock begins immediately before `Popen` and ends when `wait4` reports that exact child exited. It includes parent launch/wait overhead, loader/interpreter startup, stdin parsing, search, formatting and output. Temporary stdin/stdout/stderr files are created and input is written before the clock; the child still reads/parses the entire input. File-backed stdio avoids a pipe-buffer or parent-draining bottleneck. Output is read and checked after exit. Absolute executable paths and no `preexec_fn`/cwd override permit CPython's `posix_spawn` path on the verified host.

There is one discarded warmup launch per implementation/family followed by multiple fresh-process samples. All jobs within each warmup/measurement round are shuffled with a recorded seed, avoiding a fixed language order. Raw warmup and measured observations remain in the report; warmups do not enter summary distributions. These timings must be called process latency, never pure search time. Startup is not estimated or subtracted.

## Scope 2: preloaded repeated evaluation

Each child parses and stores the complete family's case pool, verifies all results, and performs warmup rounds before the clock. The native clock is `clock_gettime(CLOCK_MONOTONIC)`; Python uses `perf_counter_ns`. The measured loop cycles over the same ordered cases and accumulates the weighted checksum. Setup, parsing, per-case oracle checking, formatting and final cleanup are outside this interval. Normal function dispatch, indexing, result construction, checksum arithmetic and loop overhead remain inside.

A short calibration run determines repetitions separately per implementation/family to target the requested duration, capped at two million calls. All implementations still evaluate the same case distribution; the exact rounds/call counts and calibration samples are recorded. Every measured sample is a fresh child with its own warmup. Implementation/family/control jobs are shuffled for each sample round. This is a repeated, warmed workload over a fixed small pool; it does not represent cold caches or general application behavior.

The Rivet harness makes only these reversible symbol substitutions in the emitted LLVM:

| Original | Benchmark boundary | Behavior |
| --- | --- | --- |
| `main` | `rivet_bench_eval` | Permit repeated invocation from the harness |
| `rv2_read_array` | `bench_read_array` | Return one of two actual prebuilt runtime array handles |
| `rv2_return_float` | `bench_return_float` | Check/capture the finite result without formatting or stdin completion |
| `rv_cleanup` | `bench_cleanup` | Record the expected cleanup boundary; defer actual freeing until all repetitions finish |

The script reverses the substitutions and requires byte-exact recovery of the original LLVM. It also checks that the count of generated fuel call sites is unchanged. No opcode, branch, comparison, arithmetic check, array bound/type check or `rv2_tick` call is removed. The C harness includes the production `native/runtime.c`, creates real arrays with `rv2_array_make`, and resets only `rv2_steps` before each independent invocation. It verifies exactly two reads, one float return, one cleanup boundary, successful exit and no additional logical allocation per call. The median search allocates no intermediate runtime values. The real `rv_cleanup` frees the retained inputs after timing. Per-case executed tick counts are reported.

This harness is intentionally outside the compiler/runtime and is not a new production calling convention. Its boundary bookkeeping and retained safety calls contribute to Rivet's measured cost. It is inaccurate to describe this as unchecked or overhead-free algorithm timing.

Each language also has a separate **control** sample using the same outer loop/checksum but reading precomputed expected results instead of evaluating the algorithm. This indicates a harness floor and reveals when loop/checksum overhead is material. It does not include all Rivet boundary work, is not a calibrated estimator of removable overhead, and is never subtracted from the measured evaluation time. The control itself may be short, especially for C, so its distribution should not be overinterpreted.

## Memory and reporting

The runner obtains each child's own resource usage from `os.wait4(child.pid, 0)`. It does not use cumulative `RUSAGE_CHILDREN`, before/after deltas, or the maximum of earlier children. The host's `getrusage(2)` documents `ru_maxrss` in bytes on macOS; Linux's KiB value is multiplied by 1024. Both the original raw value and normalized bytes are stored. Linux support is implemented but is not claimed verified by a macOS run.

RSS is the **peak of the entire child process**, including startup, parsing/preload, temporary allocations, warmup, measured work and interpreter/runtime overhead. The ordinary-process child holds one input pair. A preloaded child holds the whole sixteen-case family plus harness state, so those RSS scopes are not interchangeable. RSS is not O(1) auxiliary search memory, not allocation counts, and not a steady-state snapshot. Python parsing can create temporary token/object storage that raises peak RSS even if later released. No baseline subtraction is performed.

The JSON report preserves all corpus vectors, expected outputs, verification records, shuffled execution orders, raw sample timings/RSS/CPU usage, calibration counts, weighted checksums, instruction counts, build commands, executable sizes/hashes, source/runtime/compiler/harness hashes, revision/working-tree state, host and tool versions, clock information and units. Summary tables use min/p10/median/p90/max distributions; the percentiles describe these samples, not statistical confidence intervals. Compilation and validation time are outside runtime comparisons. Any final prose must retain both scopes, report slower results as well as faster ones, and avoid extrapolating from this one algorithm to language-wide superiority.
