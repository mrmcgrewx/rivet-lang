# Median of Two Sorted Arrays and comparative performance

Implement [Median of Two Sorted Arrays](https://leetcode.com/problems/median-of-two-sorted-arrays/description/) in actual binary Rivet source. Use binary partitioning of the shorter array: O(log(min(m,n)+1)) search, O(1) auxiliary search storage. Reading the two arrays still costs O(m+n) time/storage. Sortedness and element bounds are caller preconditions; scanning to validate them would change the search complexity. Enforce lengths 0..1000 and nonempty combined input in the program. Previous binary examples remain byte-exact. No solver helper or LeetCode submission.

The user also requests runtime and memory comparisons with established languages. Add reproducible comparisons with the same partition algorithm in C and Python, separating ordinary executable latency from an explicitly documented preloaded-data harness. Report losses, overhead, scope and raw samples; no universal language claim.

## Additive floating-point contract

Keep RVB2 layout/limits/ABI 2; add register tag 8 `Float` (IEEE-754 binary64, finite values only). Copies preserve exact bits, including negative zero. Add these hex opcodes (register operands u16):

| Opcode | Result | Payload / semantics |
| --- | --- | --- |
| 04 | Float (8) | destination, 8 little-endian IEEE-754 bits; reject NaN/infinity at decode |
| 19 | Float | destination, Int source; signed i64 to binary64, round to nearest/ties to even |
| 1a | Float | destination, Float left, Float right; native IEEE addition, fail on nonfinite result |
| 1b | Float | destination, Float left, Float right; reject either signed zero divisor before division; fail on nonfinite result |
| 55 | void | Float source; require consumed input, print canonical scientific decimal plus newline and return |

Arithmetic uses default IEEE rounding, permits underflow/subnormals, no fast-math flags. Every operation retains fuel/failure ordering; unused invalid arithmetic fails. Return format is one leading digit, decimal point, exactly 16 fractional digits, lowercase e, explicit exponent sign and at least two exponent digits (C `%.16e`; reference normalizes exponent width/sign). This gives 17 significant digits, preserves signed zero and roundtrips binary64. Median examples therefore print `2.0000000000000000e+00` and `2.5000000000000000e+00`.

LLVM lowers literals from exact bits, conversion via sitofp, arithmetic via fadd/fdiv and direct finite/zero checks; no math helper computes median. C adds only `void rv2_return_float(double value)` for generic formatting/input completion, defensively rejects nonfinite output. Existing array/node helpers are reused. Tests must cover finite extremes, subnormals, negative zero, precision loss at 2^53/i64 boundaries, overflow/zero, copy/CFG/type/byte validation, roundtrips and reference/native agreement.

## Program and profile

`examples/median-two-sorted-arrays.rvb` reads two length-prefixed arrays, swaps array handles/lengths to search the shorter, partitions with guarded boundary loads, compares cross-partition edges and returns the larger left edge for odd total length or floating mean of the middle pair for even length. Sentinel endpoints use signed i64 extrema; problem inputs are bounded to +/-1000000. Computing the even mean converts each integer before adding, avoiding integer overflow. Example input `2 1 3 1 2\n` returns 2; `2 1 2 2 3 4\n` returns 2.5. API metadata id `median-two-sorted-arrays`, name `Median of Two Sorted Arrays`, same binary URL/filename convention.

Add explicit `speed` native profile using -O3 and dead stripping, preserving all checks and default size profile. Test all three profiles for new features/solution. Benchmarks compare -O3 Rivet and -O3 C plus the installed Python interpreter, with tool versions recorded; size/baseline retain existing meaning.

## Benchmark method

Keep benchmark machinery separate from production compiler/runtime. Use seeded sorted cases up to 1000 elements per array, empty/asymmetric/interleaved/disjoint/duplicate/negative cases, and an independent merge/sort median oracle. Same partition algorithm and observable result in all three languages.

Report two scopes: (1) executable process launch through exit with stdin parsing, output and peak process RSS; (2) repeated preloaded-data evaluation with setup/formatting excluded, monotonic elapsed time, checksums preventing dropped work, warmups, multiple randomized execution orders and distributions. The preloaded Rivet harness may rename the emitted main and substitute only input/output/cleanup boundaries to supply actual prebuilt runtime arrays and capture results. Preserve generated arithmetic, branches, bounds checks and per-instruction fuel calls. Reset fuel between independent invocations using benchmark-only access in a runtime-including harness, never by removing ticks. Free actual allocations after measurement. Document exact substitutions and verify harness outcomes against unchanged standalone executables. Report peak RSS for each child separately with platform-correct units; RSS includes interpreter/runtime/process overhead and is not auxiliary algorithm memory. Do not subtract startup estimates or label whole-process timing as pure algorithm time.

## Ownership committed before delegation

- Compiler owner: `src/language/compute.rs`, `compute_codegen.rs`, `tests/floating_point.rs`, `docs/COMPUTE-FORMAT.md`, `docs/MEDIAN-COMPILER-HANDOFF.md`. Implement direct codec/IR/validation/reference/LLVM and generic tests. No C, native build profile, examples, UI, shared docs or Git edits.
- Benchmark owner: `benchmarks/median/`, `scripts/median_benchmark.py`, `docs/MEDIAN-BENCHMARK-METHOD.md`, `docs/MEDIAN-BENCHMARK-HANDOFF.md`. Implement C/Python comparison and benchmark-only Rivet harness with raw report output and bounded runs. Wait for source/compiler when needed. No production compiler/runtime/UI/example/shared-doc/Git edits.
- UI owner: `web/`, `tests/ui/`, `docs/MEDIAN-UI-HANDOFF.md`. New example input guidance and tests: odd/fractional/negative/empty-sided cases, source/export/build unchanged across inputs, malformed input recovery, source view and native artifact. Keep other examples. No Rust/C/shared docs/Git edits.
- Coordinator: direct binary example writer/source, C generic float return/tests, speed build profile/CLI, metadata/API/algorithm tests, shared docs, review, Git, actual browser, clean reproduction and final benchmark evidence. Owners persist findings before ending. Independent review follows implementation; no concurrent Git mutations.

## Acceptance

Published examples plus deterministic random/boundary cases agree with an independent sorted-union oracle in reference execution and all native profiles. One artifact handles multiple inputs and runs copied alone. Generic floating operations are tested outside median. Benchmark results include raw samples, checksums, versions, timing scope and memory units; no benchmark-only solver enters runtime. Full Rust/strict C/UBSan/browser regressions, actual in-app browser, clean committed checkout and public CI pass. Preserve old fixture bytes and update README/spec/architecture/roadmap/state with measured capabilities and limits.
