# Two Sum — binary computation milestone

The requested outcome is a Rivet solution to [LeetCode Two Sum](https://leetcode.com/problems/two-sum/description/), including its linear-time follow-up. The committed `.rvb` file contains the algorithm. One native executable must accept different runtime arrays and targets; C support must not contain a Two Sum solver or a program dispatcher. Existing RVB1 examples remain byte-for-byte compatible.

## Design and fixed interfaces

Introduce RVB version 2 for computations with typed mutable registers and control flow. Preserve the 16-byte RVB header and source/instruction bounds (65536 bytes, 256 instructions). Version is 2, entry tag is 3, flags/reserved remain zero. Body starts with u16 register count (1..256) followed by one-byte type declarations: 2 Int, 3 Bool, 5 IntArray, 6 IntMap. Instruction records retain opcode:u8, result type:u8, payload length:u16, payload. Value-producing payloads begin with destination:u16; effect/control result tag is zero. All integers are little-endian. After the records, u16 entry PC must be zero; exact EOF is required.

| Opcode | Result | Payload |
| --- | --- | --- |
| 02 | Int | destination, i64 constant |
| 03 | Bool | destination, u8 0/1 |
| 11 | Int | destination, left, right; checked add |
| 13 | Int | destination, left, right; checked subtract |
| 14 | Bool | destination, left Int, right Int; equality |
| 15 | Bool | destination, left Int, right Int; signed less-than |
| 30 | Int | destination; read integer from stdin |
| 31 | IntArray | destination; read count-prefixed array from stdin |
| 32 | Int | destination, array; length |
| 33 | Int | destination, array, Int index; checked indexing |
| 34 | IntArray | destination, u16 count, count Int register references |
| 40 | IntMap | destination; new integer map |
| 41 | Int | destination, map, Int key, Int default; lookup |
| 42 | void | map, Int key, Int value; insert/replace |
| 50 | void | target instruction index |
| 51 | void | Bool condition, true target, false target |
| 52 | void | array; require consumed input, print array plus newline, return success |

All register/target fields are u16. Non-terminators fall through. Reject bad record lengths/tags/types/refs/targets and reachable fallthrough beyond the last record. Validate definite assignment on all reachable paths with a fixed-point CFG analysis, including loops. Unreachable instructions still require structural/type validity. Source inspection must not execute input or loops. RVB2 scalar convenience edits may be omitted initially (literal metadata None); full byte editing remains available.

Runtime input is ASCII whitespace-separated signed decimal i64 values (optional minus, no plus). `read_array` consumes length then that many elements. Length is 0..10000; `read_int` consumes one integer. Input is capped at 262144 bytes; missing, malformed, overflowing, or unconsumed input fails. Arrays have at most 10000 elements. Each executed instruction consumes one of 1000000 fuel units. Each array creation is charged 8*length bytes and each map creation 32768*24 bytes against a cumulative 64 MiB logical allocation budget in both reference and native paths. Maps have at most 10000 unique keys; overwrite does not consume a new entry. Allocations last until exit. Output is a compact decimal integer array plus newline (at most 262144 bytes). Maps use a bounded open-addressed hash table; expected constant-time lookup gives the program expected O(n) time, O(n) used entries, with fixed bounded table capacity in this implementation.

Native lowering emits direct LLVM basic blocks, typed register slots, checked arithmetic intrinsics, loads/stores/comparisons/branches, and direct generic support calls. Array-make scratch storage is allocated in function entry, not repeatedly on loop iterations. No native interpreter or encoded instruction stream is linked.

Native ABI additions (exact symbols and platform C ABI):

```c
int64_t rv2_read_int(void);
void *rv2_read_array(void);
int64_t rv2_array_len(void *array);
int64_t rv2_array_get(void *array, int64_t index);
void *rv2_array_make(const int64_t *values, uint64_t count);
void *rv2_map_new(void);
int64_t rv2_map_get(void *map, int64_t key, int64_t fallback);
void rv2_map_put(void *map, int64_t key, int64_t value);
void rv2_tick(void);
void rv2_return(void *array);
```

Reuse `rv_fail` and `rv_cleanup`. Runtime helpers do input, array/map storage, limits and output; the algorithm's scan, complement calculation, lookup decision and result construction live in `.rvb` and generated blocks.

Public Rust interfaces retain `compile_binary`, `inspect_binary`, `encode_binary`, `emit_llvm`, and `execute`. Add `EntryInfo::Compute` (serialized kind `compute`) and `CompiledProgram::execute_with_input(&str) -> Result<Execution,String>`; compute yields `Execution::Console`. `execute()` uses empty input. Preserve existing legacy interfaces and tests. Internal representation may be extended by the compiler owner.

Workbench adds a fourth example `{id:"two-sum", name:"Two Sum", url:"/api/examples/two-sum", filename:"two-sum.rvb", input:"4 2 7 11 15 9\n"}`. Existing examples need not have input. Run request adds optional `input:String`, default empty. Compute launches the native executable with that stdin; changing input does not invalidate its build. Existing console/server reject nonempty input. CLI `run`/`interpret` read bounded stdin only for compute entries. Build output remains independent of input.

## Ownership before delegation

- Compiler agent: `src/language.rs`, `src/language/`, new `tests/compute_source.rs`, `docs/COMPUTE-FORMAT.md`, `docs/TWO-SUM-COMPILER-HANDOFF.md`. Implement binary validation/roundtrip/inspection, reference execution, native LLVM and focused semantic tests. Do not edit CLI, process, workbench, C, web or shared docs.
- Runtime agent: `native/runtime.c`, `native/runtime.h`, `tests/native_runtime/`, `docs/TWO-SUM-RUNTIME-HANDOFF.md`. Implement generic ABI with strict C/UBSan tests. Preserve existing behavior; no Rust/web/shared doc edits.
- UI agent: `web/`, `tests/ui/`, `docs/TWO-SUM-UI-HANDOFF.md`. Add runtime input for compute entries, input help/examples, preserve byte editing/build validity/native views; browser coverage builds once/runs multiple inputs. No Rust/C/shared doc edits.
- Coordinator: CLI/process/workbench integration, binary authoring and example, native differential/oracle tests, shared docs, review, all Git operations and final acceptance. Agents persist findings/tests before ending and do not commit.

## Acceptance and validation

The byte-authored program scans once, checks the complement before inserting the current index (so duplicates work without reusing an element), returns found indices and returns an empty array when no pair exists outside LeetCode's guarantee. Test all three published examples, negatives/zero/duplicates/boundary values, maximum 10000-element input, seeded cases against a brute-force oracle, malformed input and language bounds. Native and reference outputs must agree under both optimization profiles. Reuse one artifact across inputs and run a copy outside the repo. Test malformed bytecode/definite assignment/loops and generic operations separately.

Run formatting, strict Clippy, complete Rust tests, C/UBSan, browser regression suite, actual in-app UI interaction and a clean-checkout build/test before completion. Update README/language/architecture/state with supported features and measured evidence. Commits are the ledger; publish coherent checkpoints using the required GitHub App identity. No LeetCode account submission is requested.
