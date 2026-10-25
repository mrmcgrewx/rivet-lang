# RVB2 compiler handoff

Implemented against the ownership and interfaces committed in `fe95cd0` / [TWO-SUM-PLAN.md](TWO-SUM-PLAN.md), on 2026-10-25. No Git commits or publication were performed by this agent. The coordinator owns combined acceptance and publication.

## Delivered

- `src/language/compute.rs`: private RVB2 computation variant, direct bounded binary decoding, static type/target/shape checks, reachable control-flow traversal and fixed-point definite-assignment validation, canonical encoder, compact decoded listing, and bounded reference execution with separate input.
- `src/language/compute_codegen.rs`: direct LLVM basic blocks, mutable typed register slots, signed overflow intrinsics, comparisons and branches, and calls to the fixed generic `rv2_*` ABI. Array-construction scratch allocas appear only in entry. Unreachable source tails emit valid LLVM without out-of-range branch labels.
- `src/language.rs`: `EntryInfo::Compute`, `execute_with_input`, and dispatch of existing public compilation/inspection/execution methods to the computation variant. Existing RVB1 IR remains intact. Nonempty runtime input is rejected for legacy console/HTTP programs.
- `src/language/binary.rs` and `native_codegen.rs`: version dispatch and private shared byte reader support; existing RVB1 behavior and canonical encoding are preserved.
- `tests/compute_source.rs`: 10 focused semantic/structural/native tests.
- [COMPUTE-FORMAT.md](COMPUTE-FORMAT.md): full byte encoding, runtime semantics, compiler invariants, limits and native boundary.

The canonical Two Sum source, CLI/process/API integration, runtime C implementation and frontend belong to the other assigned owners. The compiler does not special-case the example or perform Two Sum during compilation. It accepts and natively builds generic array echo, counted loops, map overwrite/fallback, arithmetic failure and resource-failure programs as well.

## Validation actually performed

Final combined command:

```sh
cargo test --locked --test compute_source --test binary_source --test native_codegen
cargo clippy --locked --all-targets -- -D warnings
```

Results: **31 tests passed** (10 RVB2, 13 RVB1, 8 existing native-codegen); strict Clippy passed. `cargo fmt --all` ran after the final changes. Earlier `cargo check --locked` passed with the coordinator's integrated Compute API.

The RVB2 suite includes all truncated prefixes of a valid computation, header/body/register/record bounds, bad tags and payloads, malformed references and targets, Boolean canonicalization, missing initialization across a branch or loop's first iteration, initialized in-place loop updates, reachable fallthrough rejection, structurally checked unreachable code, exact i64 input/output, ASCII whitespace variants, malformed/extra/oversized input, indexing failures, map overwrite/default semantics, add/sub overflow, fuel exhaustion and logical allocation exhaustion. A seeded 8000-case byte-mutation corpus checks no panic, exact encoding roundtrip and inspection for accepted variants.

Native differential tests compile **9 generic programs under each of the baseline and size profiles (18 native artifacts)** and compare success output or expected failure to the reference interpreter. These cover map operations, 10000 loop iterations constructing arrays, i64 array echo, branching and mutable arithmetic, an unreachable tail, checked add/sub overflow, an infinite source loop stopped by fuel, and repeated map allocation stopped by the budget. Temporary artifacts are removed. The coordinator owns the separate algorithm oracle/maximum-input suite and standalone artifact proof.

CLI smoke checks performed after integration: `inspect examples/two-sum.rvb` decoded 21 operations, and stdin `3 3 2 4 6` through `interpret` returned `[1,2]`. These supplement, rather than replace, native verification.

## Design notes and remaining acceptance

- Initialization analysis considers both edges of every branch, even constant conditions. Entry always includes an empty incoming state. This is conservative by design; no execution is needed to inspect or validate source.
- Every executed instruction costs one fuel unit. Input, map/array limits and cumulative logical allocation charges match the fixed contract. Native and reference diagnostics need not have identical wording.
- RVB2 scalar edit metadata is `None`; destination-prefixed payloads cannot accidentally be passed to RVB1 literal editors. Raw hex remains a complete edit path. The decoded register bank is grouped four declarations per line so the GUI immediately shows algorithm operations.
- Native maps are bounded open-addressing tables. Expected constant-time operations are a data-structure property, not a worst-case performance guarantee or a claim of novel compiler optimization.
- No compiler-side blocker remains from the tested scope. Actual browser acceptance, clean-checkout reproduction, shared documentation/state, full integrated tests and publication remain coordinator responsibilities.
