# Median / floating-point validation

Validation date: 2026-10-25. Design/ownership contract: `cccf624`. Primary host: macOS 26.5.2 arm64, Rust 1.90.0, Apple Clang 17.0.0 (`clang-1700.4.4.1`), Node 22.7.0, Playwright 1.64.1. Final implementation revision, clean reproduction, comparison results and public CI are recorded after the implementation checkpoint.

## Implementation and working checkout

- `examples/median-two-sorted-arrays.rvb` is 803 binary bytes, 81 instructions and 32 registers. Direct fixture regeneration leaves all five earlier source files unchanged. The algorithm uses the shorter-array partition search; sortedness and element bounds are caller preconditions. Reading stdin is linear even though search is logarithmic.
- `tests/median.rs` compares 275 pairs (19 published/edge/boundary cases plus 256 deterministic random pairs) to an independent sorted-union oracle in reference execution and all three native profiles: 825 successful native runs. It also checks malformed/missing/extra input, both-empty and oversized arrays, metadata/profile flags, and an executable-only copy with empty environment for each profile. A separate reference corpus covers all 3135 nonempty pairs of nondecreasing arrays of length 0..3 over -2..2, plus CLI negative fractional output.
- The initial integrated suite passed all 88 existing/median/API tests, formatting, strict all-target Clippy and release build. Eight generic Float tests added afterward also passed, including 27 native artifacts/255 launches across all three profiles, exact finite bits/precision/signed-zero/subnormal/failure cases and 4000 mutation/finite-bit probes. See `MEDIAN-COMPILER-HANDOFF.md`; the final clean-checkout gate must run all 96 together.
- Strict C native tests passed under baseline, size and speed, plus UBSan. Added scalar-output checks cover signed zero, maximum finite value, smallest subnormal, rejection of nonfinite values, extra/oversized input and broken output pipe, alongside all prior values/list/map/HTTP/lifecycle tests.
- All 18 Chromium scenarios passed in 35.2 seconds. The new cases verify odd/fractional/negative/empty-sided input, one-build reuse, byte-exact export, malformed input recovery and contextual help. Desktop and compact layouts were inspected; see `MEDIAN-UI-HANDOFF.md`.

## Actual in-app browser and independent artifact

The release workbench on port 7331 was refreshed. Actual in-app interaction selected the sixth example, inspected the decoded Float/register/partition operations, built once, and ran `[1,3]`/`[2]` -> 2, both-empty -> assertion failure at PC 18, empty/`[-2,1]` -> -0.5, then `[1,2]`/`[3,4]` -> 2.5. All used build #1; error recovery did not rebuild or modify source. The final tab shows hex bytes and fractional native output. Screenshot: `docs/evidence/median-two-sorted-arrays.png`.

The GUI size-profile artifact is 35336 bytes. A separate release-compiler speed-profile artifact at `.rivet/median-deliverables/median` is 34792 bytes; `file` identifies arm64 Mach-O and `otool -L` lists only libSystem. Native executable-only behavior is also exercised by the algorithm tests. Artifact sizes are observations for this host/toolchain, not a general profile ordering guarantee.

## Remaining acceptance at this checkpoint

Complete independent code/benchmark review; publish reproducible runtime/RSS results; commit implementation; verify a fresh public clone and successful public CI; update state. No LeetCode submission or whole-language performance claim is made.
