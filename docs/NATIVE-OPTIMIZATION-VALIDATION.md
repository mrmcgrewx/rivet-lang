# Native fuel optimization validation

Validation date: 2026-10-25. Contract committed before delegation: `51745ac`. Host: Apple M1 Pro, macOS 26.5.2 arm64, Rust 1.90.0, Apple Clang 17.0.0, Node 22.7.0 and Playwright 1.64.1.

## Change and diagnostic evidence

The production change exposes the existing fuel check to LLVM and adds a stable-address getter for the existing C counter. It changes no source instruction, algorithm, limit, reference semantics, runtime check, profile flag or benchmark file. All six source fixtures regenerate byte-for-byte. No LTO or fast-math is enabled.

A three-second CPU sample of the original balanced median workload placed 1373 of 2502 samples (54.9%) in `rv2_tick` and 367 (14.7%) in `rv2_array_get`. The original native evaluation function has 85 static tick calls after backend block duplication. The candidate has zero tick calls and one counter-address getter call; its four array-get call sites remain. Its instruction count grows from 250 to 493, illustrating the code-size cost of inlining, while many call barriers disappear. These are inspected native instructions, not source instruction counts. All 81 emitted source tick sites remain.

The candidate's separate diagnostic sample placed 655 of 2505 samples (26.1%) in checked array access; inlined fuel work is now part of `rivet_bench_eval`. Sampling cannot precisely separate that work from the search. It identifies remaining costs, not a safety-free performance target. `docs/evidence/native-optimization-profile.json`, the sanitized sample reports and before/after ARM64 disassemblies preserve the observations. Profiling runs are excluded from final timing evidence.

## Working checkout verification

- All 96 existing Rust tests passed in the integrated suite. Four new focused fuel tests separately passed, using real million-step source loops, exact last-allowed/first-disallowed returns, eight error classes at the limit, and shared inline/C-fallback state. They cover baseline, size and speed. The clean checkout gate will run all 100 together.
- Formatting, strict all-target Clippy and release compilation passed. All six binary fixtures regenerated exactly. Source files, the median runner and C/Python implementations/harnesses are unchanged from `b72a8bd`.
- Strict C suites passed under all three profiles and UBSan, including the new getter identity, shared state, cleanup reset and exact exhausted diagnostic checks. See the runtime handoff.
- All 18 browser acceptance scenarios passed in 29.5 seconds, including native overflow, algorithm inputs/errors/recovery, standalone downloads and HTTP lifecycle.
- An exploratory run of the unchanged benchmark validated all 128 corpus pairs and its harness checks. Its one short timing sample is diagnostic only; full clean-checkout results are required before reporting improvement.

## Actual workbench and standalone executable

The release workbench on port 7331 was restarted with the updated compiler, retaining the browser's existing median source and input. Clicking Build produced a new independent 35352-byte size-profile artifact. Clicking Run returned `2.5000000000000000e+00` for `2 1 2 2 3 4`, with exit code zero. Source remained 803 binary bytes and 81 instructions. The screenshot is `docs/evidence/native-optimization-workbench.png`; the displayed UI duration includes process launch and workbench handling and is not the preloaded benchmark metric.

A separate speed-profile executable at `.rivet/optimization-deliverables/median` is 34808 bytes and returned the same result with an empty environment. Earlier artifacts remain on disk. The compiler embeds its runtime; the executable does not depend on the workbench, compiler, source or inspection sidecars.

## Completion gates

Independent review, clean public reproduction, full before/after default benchmarks and CI are recorded at the final evidence checkpoint. No final performance claim is made by the short exploratory run above.
