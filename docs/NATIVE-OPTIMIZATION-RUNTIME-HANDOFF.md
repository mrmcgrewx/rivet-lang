# Native fuel getter handoff

Completed 2026-10-25 against `NATIVE-OPTIMIZATION-PLAN.md`, committed before delegation in `51745ac`. Changes are confined to `native/runtime.c`, `native/runtime.h`, `tests/native_runtime/compute.c`, `tests/native_runtime/run.py`, and this handoff. No Git, Rust, benchmark, UI or shared-document edits were made by the runtime owner.

## Additive interface

```c
uint64_t *rv2_fuel_counter(void);
```

The implementation is exactly `return &rv2_steps;`, exposing the stable address of the existing static counter. The getter does not read or write memory, allocate, reset, check a limit, or fail. It remains valid after `rv_cleanup`; cleanup resets the pointed-to value without changing its address. This is a runtime/compiler interface, not a new source-language operation.

The original C `rv2_tick` body and symbol are unchanged: equality with 1000000 fails with `compute instruction limit exceeded (1000000)` before increment; otherwise it increments by exactly one. Original runtime helpers and limits are untouched. The new getter is additive within ABI 2, enabling the compiler's visible fuel body to share exactly the same state as fallback calls and the existing benchmark reset.

## Checks actually performed

All passed on macOS arm64 with Apple Clang 17.0.0:

```sh
clang -std=c11 -Wall -Wextra -Wpedantic -Wconversion -Wshadow -Werror -fsyntax-only native/runtime.c
python3 tests/native_runtime/run.py
python3 tests/native_runtime/run.py --sanitize undefined
clang -std=c11 -O3 -S -emit-llvm native/runtime.c -o /tmp/rivet-runtime-fuel.ll
git diff --check -- native/runtime.c native/runtime.h tests/native_runtime/compute.c tests/native_runtime/run.py
```

The runner passed all baseline (`-O0`), size (`-Oz`) and speed (`-O3`) suites, plus UBSan. It repeats existing text/JSON, arrays/maps/lists/floats, input/output/quota/fuel and HTTP lifecycle coverage. The new focused assertions verify initial zero, pointer identity, repeated getter calls without changes, writes through the getter observed by `rv2_tick`, ticks observed through the pointer, the getter succeeding at exhaustion, and stable identity/zero after repeated cleanup.

The boundary subprocess test sets the shared counter to 999999, verifies the next tick reaches exactly 1000000 successfully, and verifies the following tick fails with exit code 1, no stdout, and the exact existing stderr line including runtime prefix/newline. A separate success mode stops after the last allowed tick. The existing million-call loop and overflow-fuel test remain.

The emitted optimized C LLVM getter body was inspected and contained only `ret ptr @rv2_steps`. Clang independently inferred `nounwind`, `willreturn` and `memory(none)` (among other attributes), supporting the proposed truthful declaration in the compiler. No broad optimization claim follows from this small helper alone.

Compiler inline-body/reference checks, independent review, profiling, controlled before/after benchmarks, clean reproduction and final publication remain with the coordinator and assigned owners. No performance measurements or speedup are claimed in this handoff.
