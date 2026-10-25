# Immutable linked-list runtime handoff

Implemented 2026-10-25 against `ADD-TWO-NUMBERS-PLAN.md`, committed before delegation in `8a6b220`. Changes are confined to `native/runtime.c`, `native/runtime.h`, `tests/native_runtime/`, and this document. The coordinator owns integration, Git operations and final acceptance.

## Native representation and behavior

The six specified ABI additions are implemented: `rv2_read_list`, `rv2_list_cons`, `rv2_list_value`, `rv2_list_next`, `rv2_list_len`, and `rv2_return_list`. The ABI remains version 2. Existing signatures and RVB1/RVB2 behavior are preserved.

Nil is NULL. A nonnil handle points to a tagged, tracked native allocation containing an i64 value, an actual pointer to its immutable tail, and a cached chain length. Cons allocates a new node and shares the existing tail without copying it. Length is O(1); nil has length zero. Reading nil's value or next fails with a useful diagnostic. No mutation operation is exposed, so validated constructors cannot form cycles. The allocation list releases all nodes iteratively at cleanup; destruction never recursively follows tails.

Each created node charges exactly 24 logical bytes against the existing shared 64 MiB cumulative budget. Cons checks the tail's type and 10000-node chain limit before charging or allocating. Shared tails incur no further charge. The budget excludes allocation headers and temporary buffers, consistent with existing array/map rules.

`read_list` follows the failure order agreed with the compiler owner:

1. Read the signed integer length and require 0..10000.
2. Charge `24 * length` before parsing any elements.
3. Parse every element using the existing bounded signed-i64 input parser.
4. Construct immutable nodes in reverse allocation order, preserving the original input sequence, without charging again.

The input helper uses a fixed bounded 10000-i64 temporary stack buffer; the returned handle is a chain of actual nodes, not an array view. A budget failure takes precedence over malformed/missing elements after an otherwise valid length. No partially parsed list is published. Length zero charges nothing and returns nil, even with an exhausted logical budget.

Return traverses the supplied nodes, checks that input was consumed before emitting output, and writes a compact decimal array plus newline. Nil prints `[]`. It preserves arbitrary signed i64 values and does not check decimal digits, reverse-order number rules, perform addition, or solve the demonstration. Generated code remains responsible for cleanup and returning from main. Output preparation and I/O checks are shared with array return, retaining the 256 KiB output limit and closed-pipe diagnostics.

## Validation actually performed

All passed on macOS arm64 with Apple Clang 17.0.0:

```sh
clang -std=c11 -Wall -Wextra -Wpedantic -Wconversion -Wshadow -Werror -fsyntax-only native/runtime.c
python3 tests/native_runtime/run.py
python3 tests/native_runtime/run.py --sanitize undefined
git diff --check -- native/runtime.c native/runtime.h tests/native_runtime
```

The runner builds the new `lists.c` fixture with strict C warnings under `-O0` and `-Oz`; UBSan runs with `-O1 -g -fsanitize=undefined -fno-omit-frame-pointer -fno-sanitize-recover=all`. All prior value, array/map, input/fuel/quota and HTTP lifecycle checks are repeated in every run. No ASan result is claimed.

New coverage includes:

- Nil length/output, nonnil type tags, head/next semantics, signed extrema, negative and nondigit values, and two different heads sharing the identical tail pointer without altering it.
- Cached lengths during construction and iterative traversal of 10000 nodes, iterative cleanup and repeated cleanup, and rejection of the 10001st node.
- Input-order preservation, multiple list reads in one stream, leading zeros/negative zero, all ASCII whitespace characters, missing/malformed/overflowing/trailing input, invalid list lengths, exact 262144-byte input and one-byte excess.
- A 10000-node list of INT64_MIN values producing exactly 210002 output bytes, plus output to a closed pipe.
- Exact shared logical budget success using maps, an array and 10922 separately allocated nodes, followed by failure on one more node. Separate white-box precharge probes establish length-before-budget and budget-before-element-parse error ordering without unnecessary large allocations; empty list read remains valid at quota.
- Nil value/next failures and wrong valid-handle type rejection. Every expected failure exits nonzero with the diagnostic and no stdout.

The runtime assumes typed handles supplied by validated generated code; it is not a safe interface for arbitrary foreign pointers. Core list operations are generic and independent of the Add Two Numbers algorithm. Native/reference end-to-end arithmetic, binary source, workbench, clean-checkout and publication acceptance remain with the coordinator.
