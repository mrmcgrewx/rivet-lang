# RVB2 computation source format

RVB2 extends Rivet with runtime input, integer arrays, integer maps, mutable typed registers, and control flow. Its canonical source is a binary `.rvb` file. The compiler directly validates its bytes and lowers the operations to native LLVM blocks. The readable listing from `rivet inspect` and the workbench is a decoded view, not an additional source language. RVB1 console/HTTP files retain their existing encoding and behavior; see [BINARY-FORMAT.md](BINARY-FORMAT.md).

The implementation contract was committed before implementation in [TWO-SUM-PLAN.md](TWO-SUM-PLAN.md). The Two Sum algorithm is one program using these general operations. No opcode or native helper solves Two Sum.

## Header and register bank

All multibyte integer fields are little-endian. Sources contain at most 65536 bytes, 256 instruction records and 256 registers. Instruction PCs and register IDs are separate zero-based indices; a register is not identified by the instruction that writes it.

| Offset | Width | Meaning |
| --- | --- | --- |
| 0 | 4 bytes | Magic `52 56 42 00` (`RVB` followed by NUL) |
| 4 | u16 | Version `2` |
| 6 | u16 | Flags `0` |
| 8 | u16 | Instruction count, `1..256` |
| 10 | u8 | Entry tag `3`, computation |
| 11 | u8 | Reserved `0` |
| 12 | u32 | Exact number of bytes after the 16-byte header |
| 16 | u16 | Register count, `1..256` |
| 18 | register-count bytes | One declared type tag for each register |

The register tags are `2` Int (signed i64), `3` Bool, `5` IntArray, and `6` IntMap. Registers begin uninitialized and keep their declared type for the whole execution. Multiple instructions can write the same register. Operations read all operands before replacing the destination, so `add %0, %0, %1` is valid when `%0` and `%1` are initialized Int registers. Arrays are immutable values; maps are mutable tables. This revision has no general reference-copy operation or user functions.

The register bank makes loop-carried values explicit without encoding textual variable names. Numeric PCs describe branching and numeric references describe data dependencies. The compiler validates type and initialization invariants before native emission. This is a design convenience for structured byte generation and editing; improved AI authoring reliability has not been measured.

## Instruction records and entry

Each instruction is `opcode:u8, result-tag:u8, payload-length:u16, payload`. Result tags agree with the destination's declared type. Operations without a destination use result tag zero. The result tag zero is not a register type. All register references, destinations, and target PCs in the following table occupy u16. Literal numeric bytes are payload values, not register references. `count` is also u16.

| Hex opcode | Result tag | Payload fields in order | Meaning |
| --- | --- | --- | --- |
| `02` | 2 | destination, signed i64 literal | Set Int constant |
| `03` | 3 | destination, u8 Boolean literal | Set Bool constant; literal must be 0 or 1 |
| `11` | 2 | destination, left, right | Checked signed addition |
| `13` | 2 | destination, left, right | Checked signed subtraction |
| `14` | 3 | destination, left, right | Int equality |
| `15` | 3 | destination, left, right | Signed Int less-than |
| `30` | 2 | destination | Read one input integer |
| `31` | 5 | destination | Read count-prefixed input array |
| `32` | 2 | destination, array | Array length |
| `33` | 2 | destination, array, index | Array element at an Int index |
| `34` | 5 | destination, count, `count` Int references | Construct an array from current register values |
| `40` | 6 | destination | Create an empty integer map |
| `41` | 2 | destination, map, key, fallback | Lookup Int key; return stored Int or fallback Int |
| `42` | 0 | map, key, value | Insert or replace an Int key/value pair |
| `50` | 0 | target PC | Unconditional jump |
| `51` | 0 | Bool condition, true PC, false PC | Conditional branch |
| `52` | 0 | array | Finish input, print array, return success |

All operations except jump, branch and return fall through to the next PC. Entry starts at PC zero. After the last declared record there is exactly one u16 entry PC, required to equal zero. There are no extra sections or trailing bytes. An array construction can contain repeated references or zero elements.

The decoder rejects unknown opcodes/types, mismatched result tags, out-of-range or wrongly typed registers, out-of-range branch targets, noncanonical Boolean values, truncated or oversized payloads, extra payload bytes, and invalid header/count/entry fields. All records receive structural and type validation, including unreachable records. Diagnostics identify absolute source byte offsets.

## Control-flow validation

The compiler traverses the control-flow graph from PC zero and rejects a reachable instruction that falls off the end. A forward fixed-point analysis computes which registers have been initialized on every incoming reachable path. PC zero always has an empty initial incoming state, including when another instruction jumps back to it. A register must be definitely initialized before every reachable read. An instruction's destination becomes initialized after its operands are read.

Both successors of a branch count as reachable, even when the condition is a constant. This deliberately conservative rule avoids execution during validation. Unreachable records may read uninitialized registers or have a non-terminating final fallthrough, but must still have valid types and targets. Infinite loops are legal programs; the execution fuel bound terminates them with an error. The compiler does not claim to prove termination or successful return.

## Runtime data and failure semantics

Input is a separate stdin stream, not source. Integer tokens consist of an optional ASCII minus and one or more ASCII digits, separated by any of space, tab, line feed, vertical tab, form feed or carriage return. A plus sign, non-ASCII spacing, embedded NUL, invalid suffix, missing token or value outside signed i64 is rejected. Leading zeros and negative zero are allowed. `read_array` first reads an integer length, then that many integers. `read_int` reads one integer. Return rejects any remaining non-whitespace input.

For the committed Two Sum program, input is `length elements... target`. For example, `4 2 7 11 15 9` represents the four values `[2,7,11,15]` and target `9`. This layout is a consequence of its `read_array` then `read_int` instructions, not a special parser for Two Sum. Other programs can read a different sequence.

| Resource/condition | Rule |
| --- | --- |
| Runtime input | At most 262144 bytes |
| Array length | `0..10000` elements |
| Array indexing | Signed index must be nonnegative and less than array length |
| Integer arithmetic | Signed i64 overflow fails, including unused results |
| Map capacity | At most 10000 distinct keys per map; replacing a key remains allowed |
| Execution fuel | At most 1000000 executed instructions; each operation, including jumps and return, costs one |
| Logical allocation | Cumulative 67108864-byte limit: array creation charges `8 * length`, map creation charges `32768 * 24` |
| Array output | Compact decimal JSON array plus newline; maximum 10000 i64 elements fits within 262144 bytes |

Failed operations terminate execution without proceeding to later instructions. The allocation limit is a defined logical budget rather than a measurement of allocator overhead. Native support retains tracked allocations until exit; reference execution retains array/map values for the same lifetime. The native integer map uses open addressing with a bounded 32768-slot table. It provides expected constant-time operations; worst-case probe behavior is not a guaranteed constant-time bound. The reference interpreter uses a Rust integer map with the same observable lookup/overwrite/capacity rules.

## Native compilation and inspection

`src/language/compute.rs` directly decodes RVB2 and implements validation, canonical encoding, decoded listings and reference execution. `CompiledProgram::execute_with_input` is the reference path; `execute()` supplies empty input. Compilation and inspection never consume input or execute operations.

`src/language/compute_codegen.rs` lowers each instruction PC to a native LLVM basic block, using typed register slots, arithmetic overflow intrinsics, comparisons, loads, stores and direct branches. Register slots and array-construction scratch storage are allocated once in function entry. The backend can optimize these slots; a loop cannot repeatedly grow the native stack through array-construction scratch allocations. A tick call enforces instruction fuel even when LLVM optimizes other operations.

The native executable calls generic C routines for stdin parsing, array/map storage, resource checks and output. It does not contain a Rivet decoder, encoded program interpreter, or Two Sum helper. Integer addition/subtraction and the algorithm's comparisons and branches are emitted LLVM operations. The exact helper ABI is in [TWO-SUM-PLAN.md](TWO-SUM-PLAN.md) and `native/runtime.h`.

Inspection retains the record and payload byte offsets used by the workbench. RVB2 literal convenience-edit metadata is currently absent because a value-producing payload begins with a destination field. Raw hex editing still supports the complete source; source exports remain the actual bytes. Decoding and re-encoding any accepted RVB2 program preserves every byte, including unreachable records and unused declarations.
