# Independent native fuel optimization review

Reviewed on 2026-10-25 against the contract committed in `51745ac89a30f6abc63916c8fff5a97d4a32f7eb` and the subsequent compiler/runtime working-tree implementation. The reviewer did not implement this optimization. This task changed only this document; probes and generated artifacts lived outside the repository. No Git mutations were made.

## Result

No unresolved correctness blocker was found in the inline-fuel candidate. It exposes the existing check to LLVM while preserving the original C fallback, counter identity, instruction limit, failure precedence and cleanup. Performance results, integrated regressions and final clean-checkout acceptance remain coordinator-owned evidence; the independent probes below establish semantic comparisons, not speedup.

## Safety argument and implementation inspection

- Every source instruction still begins with the same `call void @rv2_tick()`. The new body gets the stable C counter address, loads its current value, compares **equality** with 1000000 before increment, and either calls `rv_fail` with the existing exact 44-byte message or stores one unsigned increment. There are no `nsw`/`nuw` flags that could introduce overflow assumptions. The real C tick and all other helpers are unchanged.
- Getter attributes apply only to `rv2_fuel_counter`, whose entire C body is `return &rv2_steps;`. Returning this static address neither reads nor modifies the counter, cannot throw and always returns. `memory(none)`, `nounwind` and `willreturn` therefore describe the implementation truthfully; they do not mark counter loads/stores or the potentially failing tick as pure. The declarations retain normal memory effects for runtime operations and cleanup. The attributes and `available_externally` behavior were checked against the [LLVM language reference](https://llvm.org/docs/LangRef.html#function-attributes).
- `available_externally` supplies an optimization body without creating a second linkable definition. The unchanged C symbol remains available if the body is discarded or a call remains. The emitted code does not name the private static counter or embed an allocation layout. The probe's forced-fallback objects had an unresolved `_rv2_tick` reference and linked successfully against the production C implementation.
- The compiler diff changes only the tick declaration/body and its inclusion in the module. Source operation lowering, checked signed add/subtract/divide/remainder, finite Float and signed-zero guards, bounds checks, assertions, input completion and allocation helpers are unchanged. No fast-math, LTO, larger limit, batched fuel across failing work or solver-specific operation was introduced. The counter is shared with normal success/failure cleanup and with the unchanged benchmark harness reset.

## Independent probes performed

A temporary direct-byte writer constructed **23 generic RVB2 programs**. Their emitted LLVM was linked to the actual C runtime through a test entry wrapper. For each program, an alternate LLVM file replaced only the inline tick definition with an external declaration, forcing the original C fallback. Both variants used each existing profile's code-generation level: `-O0`, `-Oz` and `-O3`. The wrapper could prime the shared counter, invoke C ticks and observe cleanup through an exit handler; it did not replace program operations or runtime failure handling.

| Check | Observed result |
| --- | --- |
| Inline versus C fallback | **306 paired comparisons / 612 native launches** passed across all three optimization levels. Exit status, stdout and stderr matched byte-for-byte for every pair. |
| Exact limit and ordering | Tested zero, last permitted ticks, first disallowed tick, already-exhausted entry and failures immediately before/after exhaustion. A normal source loop executed exactly **1000000 instructions** and returned successfully; a longer path failed on fuel. Extra input at the last allowed return produced the input error before any later fuel failure. |
| Preserved failures | Unused signed add/subtract overflow; divide/remainder by zero and MIN/-1; nonfinite Float add/divide; positive/negative-zero divisors; array indexing; nil head/next; assertion PC; missing/malformed input; array/list length; remaining input; and actual repeated-map exhaustion of the **64 MiB logical allocation budget**. Each error occurred when its instruction was permitted, while an exhausted preceding tick won otherwise. |
| Shared state and cleanup | Mixed C/generated ticks used the same counter. Three successive calls in one process succeeded after normal cleanup. Exit handlers checked zero fuel/charge, no retained allocations/input, stable getter identity and repeated cleanup after both success and failure. |
| Equality and unsigned semantics | Test-only counter values above the limit and at `UINT64_MAX` matched C equality/wrapping behavior. These states are ABI robustness probes, not states reachable by ordinary validated programs. |
| Reference comparison | **26 normal-state cases** matched the reference interpreter's exit status and stdout, including the exact-limit loop cases. Reference/native diagnostic wording already differs in some helpers; that existing distinction was retained. Exact error-byte equivalence was checked between optimized native and C-fallback variants. |

The temporary comparator initially assumed identical reference/native wording; inspection identified existing distinctions such as “value of nil list” versus “value from nil list.” The final reference comparison retained those established diagnostics and compared status/output, while the native pair comparison remained exact. No production failure was discovered or changed for this difference.

## Unchanged comparison inputs

Byte comparisons and SHA-256 calculation verified **12 tracked files** against completed median implementation `b72a8bd`: all six `examples/*.rvb` sources, `scripts/median_benchmark.py`, and the five tracked files in `benchmarks/median/` (`median.py`, `partition.c`, `partition.h`, `preloaded.c`, `standalone.c`). Every comparison was identical. No benchmark baseline, case generation, timing/RSS collection, checksum, boundary substitution or safety-reset code was edited for the candidate.

The unchanged runner still counts source `rv2_tick` call sites and reverses only the documented benchmark boundary substitutions. Inlining changes generated machine code, which is the intended candidate; it does not change what the benchmark times or the source instructions it validates. Prior method review remains in [MEDIAN-REVIEW.md](MEDIAN-REVIEW.md).

All reviewer test/probe processes were stopped before notifying the coordinator that controlled final measurements could proceed. Temporary probe files were removed after recording these results. These bounded experiments are not a formal proof; only the existing macOS Apple Silicon target was executed. Owner tests, full regressions, actual browser verification, fresh-checkout acceptance, CI and controlled before/after timing are separate evidence and are not claimed as independently rerun here.
