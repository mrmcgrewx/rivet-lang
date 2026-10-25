# Add Two Numbers in Rivet

[`examples/add-two-numbers.rvb`](../examples/add-two-numbers.rvb) is a 657-byte binary program with 70 instructions and 17 typed registers. It implements [LeetCode Add Two Numbers](https://leetcode.com/problems/add-two-numbers/description/) using actual linked nodes and digit-by-digit carry. The two input lists and returned list store the least significant digit first. Numbers can contain 100 digits; the program never represents an entire number as an i64.

## Run it

```sh
cargo build --release --locked
./target/release/rivet build examples/add-two-numbers.rvb -o ./add-two-numbers
printf '3 2 4 3 3 5 6 4\n' | ./add-two-numbers
# [7,0,8]
printf '1 0 1 0\n' | ./add-two-numbers
# [0]
printf '7 9 9 9 9 9 9 9 4 9 9 9 9\n' | ./add-two-numbers
# [8,9,9,9,0,0,0,1]
```

Each list has its own count prefix. The first input is `3 2 4 3` followed by `3 5 6 4`: three digits representing 342, then three representing 465. The result list contains `[7,0,8]`, representing 807. Brackets are the printed serialization of the resulting list. Internally, `IntList` is a handle to a node containing a signed integer and a next-node handle, with nil marking the end.

In the workbench select **Add Two Numbers**, Build, and Run executable. Edit Runtime input to try another pair while preserving the same native build. The decoded view explains the binary instructions; Hex bytes exposes the actual program bytes. Source export always saves `.rvb`, excluding runtime input. The executable can be copied elsewhere and launched without Rivet, source files or the workbench.

## Algorithm in the binary source

| Instructions | Behavior |
| --- | --- |
| 0–19 | Read both linked lists, create nil results/constants, require each input length to be 1..100. |
| 20–26 | Continue while either list has nodes or carry is nonzero; initialize this digit's sum from carry. |
| 27–42 | If a left node exists, validate its digit and canonical final digit, add its value and advance its handle. |
| 43–58 | Do the same for the right list. |
| 59–62 | Calculate sum%10 and sum/10, prepend the digit to the temporary list, repeat. |
| 63–69 | Reverse the temporary chain with node operations and return the final linked list. |

After each addition iteration, carry contains the portion to be included in the next digit. Exhausted lists contribute zero. Since valid input digits are 0..9, the digit sum is at most 19 and carry at most 1; whole-number size never limits the calculation. The carry condition produces an extra final node when necessary. The second traversal restores the required least-significant-digit-first result order after immutable prepend construction.

Time is O(max(m,n)) and allocated list storage is O(m+n+max(m,n)). The result has at most max(m,n)+1 nodes. Immutable construction intentionally uses a temporary reversed chain plus the result chain. This favors simple sharing/lifetime semantics; it is not a claim of minimum allocation cost or superiority to other languages.

## Reusable language capabilities

The implementation extends RVB2 with `IntList`, nil/cons/head/next/length/input/output operations, a generic typed copy operation, guarded signed division and remainder, and assertions. Copy preserves reference identity: lists/arrays share immutable values, and copied map handles see the same mutable map. Lists can share tails but cannot form cycles because there is no link-mutation operation. Existing RVB1 and RVB2 files keep their bytes and behavior; an older compiler will reject newly introduced opcodes.

Compiler-generated LLVM owns the loops, carry arithmetic, comparisons, assertions and reversal. Division/remainder guard zero and the signed MIN/-1 overflow case before emitting native operations. Assertion diagnostics identify the source instruction. C support only parses generic integer lists, allocates/inspects nodes, enforces bounds and serializes the returned chain. No helper recognizes digit lists or performs addition/reversal for the solution.

The reference executor stores nodes in an arena of indices; the native implementation stores actual next pointers. Both use iterative lifetime cleanup, cached lengths, a 10,000-node chain limit and a 24-byte logical charge per new node. Shared tails incur no duplicate node charge. The existing 64 MiB logical allocation budget, 256 KiB input/output limits and execution fuel remain. [COMPUTE-FORMAT.md](COMPUTE-FORMAT.md) specifies every new record and failure rule.

## Validation and scope

The program itself rejects empty lists, lengths over 100, digits outside 0..9, and a final zero digit on a multi-node list. Generic input support rejects malformed, missing, overflowing or unconsumed tokens. Other Rivet programs can use full signed-i64 list values and empty lists; the decimal-digit restrictions belong to this binary algorithm.

[`tests/add_two_numbers.rs`](../tests/add_two_numbers.rs) compares 270 cases—including 256 seeded random pairs and 100-digit boundaries—against Python arbitrary-precision integer addition as an independent test oracle. Each case also runs through the Rivet reference executor and both native compiler profiles. Python is used only by this test oracle; it is not part of the built executable or runtime. Separate generic tests exercise copies/aliases, shared tails, nil failures, arithmetic guards, allocation bounds and deep cleanup. [Validation evidence](ADD-TWO-NUMBERS-VALIDATION.md) records actual results.

This exercise advances general linked-data and arithmetic capability. Functions, modules, reusable libraries, broader IO and measured agent-authoring/runtime comparisons remain future work in [ROADMAP.md](ROADMAP.md).
