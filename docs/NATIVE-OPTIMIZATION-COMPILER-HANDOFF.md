# Inline fuel compiler candidate handoff

Implemented on 2026-10-25 against [NATIVE-OPTIMIZATION-PLAN.md](NATIVE-OPTIMIZATION-PLAN.md), committed in `51745ac` before delegation. This contribution changes only `src/language/compute_codegen.rs`, `tests/native_safety.rs` and this handoff. No Git, runtime, source-format, reference, benchmark, UI or shared-document mutations were performed by this owner.

## Candidate and safety argument

The emitter still puts exactly one `call void @rv2_tick()` at each source PC, before that operation. It now supplies an `available_externally` definition marked `alwaysinline`, exposing the exact existing C tick body to LLVM while retaining the real C function as the linkable fallback.

The body obtains `uint64_t *` through `rv2_fuel_counter`, loads the current value, rejects equality with 1000000, and otherwise adds and stores exactly one. The increment has no `nuw`/`nsw` assumptions. On exhaustion it calls `rv_fail` with the exact existing 44-byte message, `compute instruction limit exceeded (1000000)`. No step is batched, removed or moved by the source emitter, and no resource threshold or error path changes.

The getter declaration alone has `nounwind willreturn memory(none)`. The runtime owner implements it as returning the address of the same stable static counter, without reading or changing its contents, allocation, failure or control flow. Returning an address does not read the stored counter. The inline tick itself retains ordinary memory reads/writes and can fail; it is not marked pure. LLVM sees no private C global name, runtime structure offsets or object layout. C fallback, inline code, cleanup and the unchanged benchmark reset all use the same storage.

The coordinator requested a stable production edit before exploratory measurement. After `cargo check` and an actual native median smoke succeeded, the owner notified the coordinator and made no further production-code edits while adding tests/documentation. This handoff does not claim benchmark performance; the coordinator owns controlled measurement and attribution.

## Validation performed

```sh
cargo check --locked
cargo test --locked --test native_safety
cargo clippy --locked --all-targets -- -D warnings
rustfmt --edition 2024 --check src/language/compute_codegen.rs tests/native_safety.rs
```

All passed. The four new tests completed in approximately 26 seconds. They cover:

1. **Source instrumentation contract:** Two Sum, Add Two Numbers and Median still have one unchanged tick call per decoded instruction, the inline body uses the stable getter, and no private `rv2_steps` symbol appears in emitted LLVM.
2. **Exact natural instruction boundary:** Binary source loops, with count derived independently from their setup/branch/subtract/jump sequence, return successfully on instructions 999999 and 1000000; attempting return at 1000001 fails with the exact native fuel diagnostic. Reference outcomes agree. All three native profiles pass.
3. **Failure ordering:** Assertions, unused integer overflow, unused nonfinite Float arithmetic, signed-zero Float division, array bounds, nil list access, missing input and allocation exhaustion still fail with their language diagnostic when they are instruction 1000000. The same operation at 1000001 instead fails on fuel before it executes. Assertion failure immediately before the final allowed tick is also covered. Reference and baseline/size/speed outcomes agree; error tests use table-driven shared source construction.
4. **Mixed inline/fallback ABI:** A small test-only entry/cleanup observer links the actual C runtime. It checks initial zero, stable pointer identity, a C tick followed by generated ticks, cleanup resetting the same counter, inline execution after the last permitted C tick and a C tick after the last permitted inline tick. The probe runs both ordinary emitted LLVM and a test-only variant that replaces the inline definition with a declaration, forcing the real C fallback. Both variants pass under all three profiles, including byte-exact failure diagnostics.

The test suite builds **66 temporary executables**: 9 exact-boundary artifacts, 51 failure-order artifacts, and 6 mixed-ABI variants. They perform **78 native launches**, plus reference evaluations of the boundary programs. Temporary source/runtime/object/artifact files are removed by the test directory guard. Test-only substitutions affect only the independent ABI probe; production compiler behavior and benchmark files are unchanged.

An ordinary CLI native run of median input `2 1 2 2 3 4` also returned `2.5000000000000000e+00` after the production edit. No test process remains running at handoff.

## Remaining ownership

No compiler-side blocker remains from these checks. The coordinator and independent reviewer own full regression, the unchanged before/after benchmark, controlled idle measurement windows, actual UI acceptance, clean-checkout reproduction and publication. The runtime owner's getter tests separately verify C identity/counter/reset behavior. Optimization may alter machine instructions only where ordinary LLVM semantics preserve the exact source fuel and failure behavior; the source contract and reference interpreter remain unchanged.
