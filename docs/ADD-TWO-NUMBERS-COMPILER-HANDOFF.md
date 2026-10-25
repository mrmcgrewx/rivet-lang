# Linked-list compiler extension handoff

Implemented on 2026-10-25 against [ADD-TWO-NUMBERS-PLAN.md](ADD-TWO-NUMBERS-PLAN.md), committed in `8a6b220` before delegation. The compiler owner changed only the assigned compute modules, linked-list test file and format/handoff documentation. No Git mutations were performed.

## Delivered

- `src/language/compute.rs` now supports IntList tag 7, generic same-type copy, signed division/remainder, assertions, nil/cons/nil-test/value/next/read/length operations and list return. All new operations participate in typed operand checking, exact payload decoding, canonical encoding, readable disassembly, destination/read tracking and CFG definite-assignment validation. Both return forms terminate control flow; assertions remain potentially failing fallthrough operations.
- Reference lists are nullable indices into a flat node arena. Each immutable node holds a signed i64, a tail index and cached length. Copying a list shares its tail; copying a map shares the mutable table. Array copies share the immutable value. No copy charges or clones underlying collections. Arena destruction is nonrecursive.
- List cons checks its tail length before charging 24 logical bytes and constructing a node. Read-list checks length, charges the full `24 * count` before consuming elements, parses all values, then builds nodes in reverse construction order without another charge. Input order and failure ordering match the runtime owner's agreed C behavior.
- `src/language/compute_codegen.rs` lowers nil to null, nil-test to pointer comparison, copy to typed load/store and assertions to direct branches. Failed native assertions identify the exact source instruction PC. Division/remainder explicitly branch away from zero and MIN/-1 before emitting sdiv/srem, avoiding undefined behavior even for remainder. Generic list helpers provide storage/input/output; no arithmetic or solver helper was introduced.
- [COMPUTE-FORMAT.md](COMPUTE-FORMAT.md) documents all additive opcodes, types, aliasing, list lifetime/input/bounds, arithmetic guard semantics and native boundary. Existing RVB1/RVB2 source encodings and public APIs remain intact.

## Verification actually performed

```sh
cargo check --locked
rustfmt --edition 2024 src/language/compute.rs src/language/compute_codegen.rs tests/linked_lists.rs
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --test linked_lists --test compute_source --test binary_source --test native_codegen
```

All commands passed. The final targeted run passed **40 tests**: 9 linked-list tests, 10 existing compute tests, 13 RVB1 binary tests and 8 existing native-codegen tests. Strict all-target Clippy passed after the final source/test changes.

The new tests cover all current typed copies (including shared mutable map updates), shared immutable list tails, in-place list advance, nil copies/dereference errors/zero length, assertion ordering and native PC diagnostics, generic signed i64 list I/O, both arithmetic signs, i64 boundaries, zero-divisor and MIN/-1 failures for both division and remainder, decoder record/tag/type/ref errors, new-operation definite assignment, list-return termination and 4000 seeded mutation/roundtrip probes plus every truncated prefix of the new example.

A generic 10000-node list echo runs reference construction/return/destruction on a **128 KiB thread stack**, demonstrating flat rather than recursive node lifetime management. Native baseline and size builds preserve all **210002 output bytes** for 10000 INT64_MIN nodes. Full-chain cons fails; a shared map/list logical-budget test fails before reading missing tokens, validating the specified read-list charge order.

The new native differential test builds **10 generic programs under each profile (20 artifacts, 58 launches)**: all-type copies, shared-tail traversal, list echo, signed division, signed remainder, full-chain cons failure, shared quota failure, nil head/next failures and assertion failure. One artifact is reused across the arithmetic/input cases. Results are compared to independently asserted reference semantics; the temporary artifact directory is removed afterward. Existing compute native differential tests also passed.

CLI smoke checks: the coordinator-authored 657-byte, 70-operation `examples/add-two-numbers.rvb` decodes successfully, and input `3 2 4 3 3 5 6 4` through the reference CLI returns `[7,0,8]`. The coordinator owns algorithm-wide native/oracle verification and independence evidence.

## Handoff and limits

No compiler blocker remains from the tested scope. Actual browser verification, full integrated regression, algorithm oracle tests, C/UBSan evidence, clean checkout, shared docs/state and publication remain coordinator responsibilities. Lists are immutable i64-only nodes with a 10000-node chain limit, process lifetime and the existing logical allocation/fuel bounds; this milestone does not add user-defined records, mutation, general functions or garbage collection. The extension remains RVB2: earlier files keep their bytes, while older compiler builds correctly reject the new capabilities they do not understand.
