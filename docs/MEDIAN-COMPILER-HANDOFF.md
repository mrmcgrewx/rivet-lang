# Finite Float compiler extension handoff

Implemented on 2026-10-25 against [MEDIAN-PLAN.md](MEDIAN-PLAN.md), committed in `cccf624` before delegation. This compiler contribution changes only `src/language/compute.rs`, `src/language/compute_codegen.rs`, `tests/floating_point.rs`, the format specification and this handoff. No Git mutations were performed.

## Delivered

- RVB2 register tag 8 (`Float`), binary64 literal `04`, signed conversion `19`, addition `1a`, checked division `1b`, and Float return `55`. Literal NaN and infinities are rejected directly from source bytes, including unreachable records. Literal bits are retained as u64 in private IR so encoding never changes negative zero or finite payloads.
- All Float destinations/reads/returns participate in type checking, definite assignment and CFG validation. Generic typed copy now supports Float and preserves exact bits. Readable inspection shows both canonical scientific decimal and hexadecimal literal bits; RVB2 source remains actual bytes with no text frontend.
- Reference execution uses finite f64 values, signed i64 conversion, default IEEE addition/division, a check for either signed-zero divisor and a finite-result check before assignment. Subnormals, signed zero and finite underflow are supported. Unused invalid operations still fail. Float return checks input completion and prints 17 significant digits, normalizing exponent sign and minimum width to the C `%.16e` contract.
- Native lowering uses exact hexadecimal double constants, `sitofp`, `fadd` and `fdiv` with no fast-math flags. An explicit zero-divisor branch precedes division. Bitcast/exponent-mask checks reject nonfinite results before their stores. Only generic formatting/input completion calls C `rv2_return_float(double)`; all arithmetic remains generated LLVM.
- [COMPUTE-FORMAT.md](COMPUTE-FORMAT.md) now specifies the additive bytes, precision/rounding, signed zero, finite-only arithmetic, output formatting and current Float limitations.

## Verification actually performed

```sh
cargo check --locked
rustfmt --edition 2024 src/language/compute.rs src/language/compute_codegen.rs tests/floating_point.rs
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --test floating_point --test linked_lists --test compute_source --test binary_source --test native_codegen
```

All commands passed. The final targeted run passed **48 tests**: 8 Float, 9 linked-list, 10 original compute, 13 RVB1 binary and 8 existing native-codegen. Strict all-target Clippy passed with the final source/test changes. This is compiler regression evidence; the coordinator owns the full integrated suite.

The Float tests cover exact literal/copy roundtrips, +0/-0, finite maxima, smallest normal, smallest/largest subnormals, default underflow, exponent formatting, checked nonfinite results, either signed-zero divisor, conversion precision around 2^53 and both signed-i64 bounds, ties-to-even, malformed/truncated/incorrectly typed operands and payloads, unreachable nonfinite literals, missing initialization, in-place float loops, and Float-return termination. Seeded probes test **4000 finite-bit/mutated-source iterations** plus every truncated prefix of the multi-literal source; accepted mutated sources preserve bytes and inspection validity.

Native differential coverage builds **9 generic programs under baseline, size and speed profiles (27 artifacts, 255 launches)**. Runtime selectors reuse artifacts across 40 exact literal bit patterns, 12 integer-conversion boundaries, 11 addition cases and 16 division cases. Additional programs prove unused nonfinite/zero-divisor operations fail under optimization, negative zero survives copy/return, extra input fails, loop-carried Float values work, and an unreachable Float tail is not evaluated. Exact native output matches the reference scientific representation; expected bit patterns and known decimal spellings are independently asserted by focused tests. Temporary artifacts are removed.

The coordinator-authored 803-byte, 81-operation median source passes direct decode and CFG validation. CLI smoke checks performed: the odd sample `2 1 3 1 2` through reference interpretation prints `2.0000000000000000e+00`; the even sample `2 1 2 2 3 4` through the ordinary native `run` path prints `2.5000000000000000e+00`. No median-specific compiler/runtime solver was introduced.

## Handoff and limits

No compiler blocker remains from the tested scope. The coordinator owns generic C return validation, algorithm oracle tests, speed-profile integration, full regression/C/UBSan/browser checks, clean reproduction, actual UI verification, benchmark review and publication. No performance claim follows from these correctness tests.

Float currently supports finite literals, copy, integer conversion, addition, division and output. It does not add floating input parsing, comparisons, arbitrary precision, NaN/infinity, or implicit coercions. Default IEEE rounding and the explicit failure checks remain part of semantics under all profiles; benchmark-only boundary substitutions must not remove them or the instruction fuel calls.
