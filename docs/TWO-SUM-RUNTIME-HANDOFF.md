# RVB2 native support handoff

Implemented 2026-10-25 against the interfaces and ownership committed in `fe95cd0` / `TWO-SUM-PLAN.md`. Changes are confined to `native/runtime.c`, `native/runtime.h`, `tests/native_runtime/`, and this handoff. The coordinator owns integration, publication and final acceptance.

## Implementation

The C ABI now exposes all ten `rv2_*` helpers specified in the plan. These helpers provide generic input, integer-array and integer-map storage, output, and runtime limits. There is no Two Sum solver, program decoder, opcode dispatcher or user-program loop in the runtime. Generated code must perform its own arithmetic, comparisons, branches and loop, call `rv2_tick` for each executed instruction, and call `rv_cleanup` on normal return. `rv2_return` prints the supplied array; generated code performs the actual return.

- Stdin is lazily loaded to EOF on the first read or return, with a 262144-byte cap. The parent must finish writing and close stdin. Tokens are parsed incrementally with exact ASCII whitespace, optional minus and decimal digits. Checked unsigned magnitude accumulation accepts the full signed i64 range, including INT64_MIN, while rejecting overflow, plus signs, embedded NUL, non-ASCII whitespace and malformed numeric tokens. Return rejects remaining nonwhitespace input before writing output.
- Arrays contain copied immutable i64 elements, with a 10000-element limit and checked signed indices. Length zero is valid. Array output preserves exact decimal integers and fits in 262144 bytes; a maximal INT64_MIN array produces 210002 bytes. Output is prepared before writing and reports closed-pipe failure instead of terminating via SIGPIPE.
- Maps use 32768 fixed slots, explicit occupancy, unsigned SplitMix64 finalization and bounded linear probing. Every i64 key and value is representable. The limit is 10000 unique keys; replacing an existing key at the limit succeeds. Missing lookup returns the caller-supplied fallback. Collision search has a hard probe bound.
- Logical allocation charging matches the language contract: 8 bytes per array element and 786432 bytes per map creation, cumulatively limited to 67108864. This is a logical language budget, not a measurement or strict cap on allocator metadata and temporary input/output buffers. The existing tracked, aligned allocation list owns array/map memory and frees it on normal cleanup or runtime failure.
- Exactly 1000000 ticks succeed; the next fails. Cleanup releases compute allocations/input and resets quota/fuel/cursors. Existing RVB1 text/JSON/HTTP behavior and its limits are retained.

The ABI assumes pointers supplied by validated generated code. Type tags catch mismatched valid value handles; it is not an interface for arbitrary foreign pointers. HTTP support remains unchanged. Compile-time assignment/type/control-flow validation and arithmetic overflow checks belong to the compiler, not these helpers.

## Validation actually performed

All passed on Apple Clang 17.0.0, macOS arm64:

```sh
clang -std=c11 -Wall -Wextra -Wpedantic -Wconversion -Wshadow -Werror -fsyntax-only native/runtime.c
python3 tests/native_runtime/run.py
python3 tests/native_runtime/run.py --sanitize undefined
git diff --check -- native/runtime.c native/runtime.h tests/native_runtime
```

The runner compiles independent C fixtures under strict warnings and both `-O0` and `-Oz`; UBSan uses `-O1 -g -fsanitize=undefined -fno-omit-frame-pointer -fno-sanitize-recover=all`. Every run also repeats the previous text/JSON, HTTP framing, listener conflict, shutdown, incomplete-client cancellation and rebind suite. No ASan success is claimed; the earlier host initialization problem remains documented in `NATIVE-RUNTIME-HANDOFF.md`.

New checks cover immutable copies, empty arrays, full signed extrema, all six ASCII whitespace characters, leading zeros/negative zero, array followed by scalar input, 10000-element input/output, exact 262144-byte input, one-byte excess, missing/malformed/overflowing/unconsumed input, invalid array lengths, both lower/upper index failures, map extrema/zero, 32 deliberately colliding keys and overwrites, 10000-key maps and replacement at capacity, the 10001st key failure, exact logical allocation-budget success then failure, map-budget failure, exact fuel success then failure, wrong handle type, repeated cleanup/reset, and output to a closed pipe. Each helper failure must exit nonzero with the expected diagnostic and no stdout.

## Integration review

Read-only review of coordinator changes in `src/native_process.rs` and `tests/two_sum.rs` found no blocking issue. The stdin writer runs independently from timeout/output draining, closes its pipe after writing, and child failure preserves its stderr ahead of any broken-pipe delivery error. Timeout/error paths kill and reap before joining threads. The runner's 262144-byte capture is sufficient for the specified compute output bound.

Two additional process-level regressions were suggested to the coordinator: a generic array echo producing more than the previous 131072-byte capture cap, and a fuel-failing program that never reads a full stdin pipe. The C harness already covers the corresponding helper output/fuel limits. Native/reference algorithm, CLI/API/UI, clean-checkout and final publication evidence remain the coordinator's responsibility.
