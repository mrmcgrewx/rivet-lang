# Two Sum in Rivet

The program is [`examples/two-sum.rvb`](../examples/two-sum.rvb): 231 bytes, 21 instructions, 13 typed registers. It solves [LeetCode Two Sum](https://leetcode.com/problems/two-sum/description/) for arrays of 2–10,000 integers within ±1 billion and a target in the same range, given exactly one pair. It returns zero-based indices and never uses one element twice. Outside the problem's one-answer guarantee, it returns the first pair found during the scan or `[]` if there is no pair.

## Running and inspecting

```sh
cargo build --release --locked
./target/release/rivet build examples/two-sum.rvb -o ./two-sum
printf '4 2 7 11 15 9\n' | ./two-sum
./target/release/rivet inspect examples/two-sum.rvb
```

The result is `[0,1]`. Input consists of an array length, that many elements, then the target. Newlines and ASCII whitespace separate numbers; finish stdin with EOF. `3 3 2 4 6` means `[3,2,4]` with target 6 and returns `[1,2]`. `2 3 3 6` returns `[0,1]`, demonstrating duplicate handling. Input parsing is a generic language operation, independent of this algorithm. The `.rvb` file remains unchanged for every input.

Choose Two Sum in the GUI, Build once, then edit Runtime input and Run repeatedly. The decoded source shows the instruction stream; the hex view edits actual source bytes. Downloads preserve the 231-byte program. Readable instruction names and the explanation here are inspection/documentation, not additional source required by the compiler.

## Algorithm encoded by the bytes

| Instruction positions | Program behavior |
| --- | --- |
| 0–6 | Read the array and target; create an empty map; initialize index, length, constants. |
| 7–8 | Compare index with array length; branch to the loop body or empty result. |
| 9–11 | Load the current element, subtract it from target, look up that complement's previous index. |
| 12–13 | Branch depending on whether the lookup returned the missing sentinel (`-1`). |
| 14–16 | Record current value → index, increment the index, jump back to the condition. |
| 17–18 | Construct and return `[previous_index, index]`. |
| 19–20 | Construct and return `[]` when no pair exists. |

Before each iteration the map contains only earlier elements. Looking up before insertion guarantees distinct indices, including equal values such as `[3,3]`. If a pair exists, when its later element is reached, the earlier value is already in the map. Each iteration performs one lookup and at most one insertion. These hash operations give expected O(n) time, with O(n) occupied entries. The prototype reserves a fixed 32,768-slot table (786,432 charged bytes) for each map to bound capacity and probing; its allocation does not currently grow with input size. This is an algorithmic complexity claim, not a measured comparison against other languages.

## Compiler and runtime boundary

RVB2 introduces typed mutable registers and control flow while preserving the earlier RVB1 console/server programs. The compiler validates types, branch destinations and assignment on every reachable incoming path before emitting LLVM. Arithmetic uses checked i64 operations. Each binary instruction becomes a native basic block with direct branches, loads/stores or helper calls. Clang optimizes and emits Apple Silicon machine code.

The C helpers implement signed-integer input, array construction/indexing, a general integer map, output and resource limits. They do not contain the scan, complement calculation, decision, or result indices. There is no specialized `two_sum` opcode, native solver, embedded program interpreter, or precomputed answer. A copied executable runs with only system libraries, without Rivet, the `.rvb` file, sidecars or workbench.

The optional [`write_binary_examples.py`](../scripts/write_binary_examples.py) packs the documented numeric records directly. It neither evaluates test data nor translates a JSON/text program. The committed binary is what Build reads.

## Limits and verification

Signed i64 arithmetic is checked. Inputs outside LeetCode's numeric domain can overflow the complement calculation and then fail rather than wrap. Lengths above 10,000, malformed or missing numbers, extra input and resource exhaustion fail with diagnostics. Programs consume at most one million instructions; the maximum-sized successful example is well below that limit.

[`tests/two_sum.rs`](../tests/two_sum.rs) compares the reference execution against an independent brute-force oracle, then runs the same inputs through the actual native artifact under baseline and size profiles. It checks all three published cases, negatives, zeros, repeated values, boundary values, no-pair extensions, 96 seeded unique-answer cases, and a 10,000-element case with more than a pipe buffer of input. It also checks malformed input, CLI stdin and standalone executable copies. Generic bytecode/control-flow/runtime tests are separate so the language is tested beyond the single solution. Recorded results live in [TWO-SUM-VALIDATION.md](TWO-SUM-VALIDATION.md).
