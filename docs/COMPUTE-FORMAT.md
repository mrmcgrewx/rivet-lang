# RVB2 computation source format

RVB2 extends Rivet with runtime input, integer arrays, integer maps, immutable linked lists, finite binary64 floating-point values, mutable typed registers, and control flow. Its canonical source is a binary `.rvb` file. The compiler directly validates its bytes and lowers the operations to native LLVM blocks. The readable listing from `rivet inspect` and the workbench is a decoded view, not an additional source language. RVB1 console/HTTP files retain their existing encoding and behavior; see [BINARY-FORMAT.md](BINARY-FORMAT.md).

The implementation contract was committed before implementation in [TWO-SUM-PLAN.md](TWO-SUM-PLAN.md). The additive linked-list, copy, division/remainder and assertion extension is specified in [ADD-TWO-NUMBERS-PLAN.md](ADD-TWO-NUMBERS-PLAN.md). The finite Float extension is specified in [MEDIAN-PLAN.md](MEDIAN-PLAN.md). Existing RVB2 files retain their exact bytes and semantics; older compilers reject operations/types they do not implement. Two Sum, Add Two Numbers and Median of Two Sorted Arrays are programs using general operations. No opcode or native helper solves these problems.

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

The register tags are `2` Int (signed i64), `3` Bool, `5` IntArray, `6` IntMap, `7` IntList, and `8` Float (finite IEEE-754 binary64). Registers begin uninitialized and keep their declared type for the whole execution. Multiple instructions can write the same register. Operations read all operands before replacing the destination, so `add %0, %0, %1` is valid when `%0` and `%1` are initialized Int registers. Arrays and list nodes are immutable values; maps are mutable tables. `copy` preserves a scalar value or shares a handle. Copies of a map refer to the same mutable table, so updates are visible through each alias. Copying an array/list handle does not clone or charge its elements/nodes. Float copies preserve all 64 bits, including negative zero. IntList nil is an initialized nullable handle, distinct from an uninitialized register. User functions remain unsupported.

The register bank makes loop-carried values explicit without encoding textual variable names. Numeric PCs describe branching and numeric references describe data dependencies. The compiler validates type and initialization invariants before native emission. This is a design convenience for structured byte generation and editing. A small [same-model authoring pilot](AGENT-EXPERIMENT-RESULTS.md) produced correct binary programs on first submission, alongside first-pass C/Rust solutions; it does not establish improved AI authoring reliability.

## Instruction records and entry

Each instruction is `opcode:u8, result-tag:u8, payload-length:u16, payload`. Result tags agree with the destination's declared type. Operations without a destination use result tag zero. The result tag zero is not a register type. All register references, destinations, and target PCs in the following table occupy u16. Literal numeric bytes are payload values, not register references. `count` is also u16.

| Hex opcode | Result tag | Payload fields in order | Meaning |
| --- | --- | --- | --- |
| `02` | 2 | destination, signed i64 literal | Set Int constant |
| `03` | 3 | destination, u8 Boolean literal | Set Bool constant; literal must be 0 or 1 |
| `04` | 8 | destination, 8 little-endian binary64 bits | Set finite Float constant; reject NaN and either infinity |
| `11` | 2 | destination, left, right | Checked signed addition |
| `13` | 2 | destination, left, right | Checked signed subtraction |
| `14` | 3 | destination, left, right | Int equality |
| `15` | 3 | destination, left, right | Signed Int less-than |
| `16` | destination type | destination, source | Copy a scalar or handle; both declared types must match |
| `17` | 2 | destination, left, right | Signed division truncating toward zero |
| `18` | 2 | destination, left, right | Signed remainder with the dividend’s sign |
| `19` | 8 | destination, Int source | Convert signed i64 to binary64; nearest/ties-to-even rounding |
| `1a` | 8 | destination, Float left, Float right | IEEE addition; reject a nonfinite result |
| `1b` | 8 | destination, Float left, Float right | Reject either signed-zero divisor; IEEE division, rejecting a nonfinite result |
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
| `53` | 0 | list | Finish input, print list values as an array, return success |
| `54` | 0 | Bool condition | Fail with `assertion failed` when false |
| `55` | 0 | Float | Finish input, print canonical scientific decimal and return success |
| `60` | 7 | destination | Initialize a nil list handle |
| `61` | 7 | destination, Int value, list tail | Construct one immutable node sharing its tail |
| `62` | 3 | destination, list | Test whether a handle is nil |
| `63` | 2 | destination, list | Read head value; nil fails |
| `64` | 7 | destination, list | Read next handle; nil fails |
| `65` | 7 | destination | Read a count-prefixed list, preserving input order |
| `66` | 2 | destination, list | Read cached chain length; nil has length zero |

All operations except jump, branch and the three return forms fall through to the next PC. A successful assertion falls through; a failed assertion exits without executing later operations. Entry starts at PC zero. After the last declared record there is exactly one u16 entry PC, required to equal zero. There are no extra sections or trailing bytes. An array construction can contain repeated references or zero elements.

The decoder rejects unknown opcodes/types, mismatched result tags, out-of-range or wrongly typed registers, out-of-range branch targets, noncanonical Boolean values, nonfinite Float literal bits, truncated or oversized payloads, extra payload bytes, and invalid header/count/entry fields. All records receive structural and type validation, including unreachable records. Diagnostics identify absolute source byte offsets.

## Control-flow validation

The compiler traverses the control-flow graph from PC zero and rejects a reachable instruction that falls off the end. A forward fixed-point analysis computes which registers have been initialized on every incoming reachable path. PC zero always has an empty initial incoming state, including when another instruction jumps back to it. A register must be definitely initialized before every reachable read. An instruction's destination becomes initialized after its operands are read.

Both successors of a branch count as reachable, even when the condition is a constant. This deliberately conservative rule avoids execution during validation. Unreachable records may read uninitialized registers or have a non-terminating final fallthrough, but must still have valid types and targets. Infinite loops are legal programs; the execution fuel bound terminates them with an error. The compiler does not claim to prove termination or successful return.

## Runtime data and failure semantics

Input is a separate stdin stream, not source. Integer tokens consist of an optional ASCII minus and one or more ASCII digits, separated by any of space, tab, line feed, vertical tab, form feed or carriage return. A plus sign, non-ASCII spacing, embedded NUL, invalid suffix, missing token or value outside signed i64 is rejected. Leading zeros and negative zero are allowed. `read_array` and `read_list` first read an integer length, then that many integers. Both preserve element order; read-list builds actual singly linked nodes. `read_int` reads one integer. All return forms reject any remaining non-whitespace input before writing output. Input tokens currently remain integers; programs obtain Float values from binary literals or signed integer conversion.

For the committed Two Sum program, input is `length elements... target`. For example, `4 2 7 11 15 9` represents the four values `[2,7,11,15]` and target `9`. This layout is a consequence of its `read_array` then `read_int` instructions, not a special parser for Two Sum. For Add Two Numbers, two `read_list` operations consume `first-length first-values... second-length second-values...`. Decimal-digit validity and the problem’s length/no-leading-zero constraints are checked by that binary program using comparisons and assertions. Generic list input accepts arbitrary i64 values, including negative values, zero-length lists and up to 10000 nodes. Median of Two Sorted Arrays reads two count-prefixed integer arrays and returns a Float. Other programs can read a different sequence.

| Resource/condition | Rule |
| --- | --- |
| Runtime input | At most 262144 bytes |
| Array length | `0..10000` elements |
| List chain length | `0..10000` nodes; cons onto a full chain fails before allocation |
| Array indexing | Signed index must be nonnegative and less than array length |
| Integer arithmetic | Signed i64 overflow fails, including unused results; division and remainder also reject zero divisors and `i64::MIN` with divisor `-1` |
| Float arithmetic | Default IEEE binary64 rounding; underflow/subnormals and signed zero are supported; nonfinite results and either signed-zero divisor fail |
| List dereference | Head/next on nil fail; nil-test and length are valid |
| Assertion | False fails with a diagnostic identifying its instruction PC |
| Map capacity | At most 10000 distinct keys per map; replacing a key remains allowed |
| Execution fuel | At most 1000000 executed instructions; each operation, including jumps and return, costs one |
| Logical allocation | Cumulative 67108864-byte limit: array creation charges `8 * length`, map creation charges `32768 * 24`; each newly allocated list node charges 24 bytes |
| Array/list output | Compact decimal JSON array plus newline; maximum 10000 i64 values fits within 262144 bytes; nil prints `[]` |
| Float output | One leading digit, decimal point, exactly 16 fractional digits, lowercase `e`, explicit exponent sign and at least two exponent digits, then newline |

Float output has 17 significant digits, sufficient to roundtrip every finite binary64 value. For example, `2` prints `2.0000000000000000e+00`, `2.5` prints `2.5000000000000000e+00`, and negative zero prints `-0.0000000000000000e+00`. Native formatting uses C `%.16e` in the default C locale; reference formatting normalizes exponent sign and width to the same contract. A Float literal preserves its exact stored bits through decoding and canonical re-encoding. Readable inspection includes both scientific decimal and the hexadecimal bits.

Signed integer conversion rounds to nearest with ties to even; integers beyond the binary64 exact-integer range may lose precision. For example, converting `9007199254740993` (2^53 + 1) yields the same Float as 2^53, while converting 2^53 + 3 rounds to 2^53 + 4. Converting i64::MAX produces the binary64 value 2^63. Float addition/division similarly round after each operation. There are no fast-math flags, implicit integer-to-Float coercions, Float input parsing, Float comparisons, NaN values or infinities in this revision. Underflow to a subnormal or signed zero is a valid finite result.

Failed operations terminate execution without proceeding to later instructions. The allocation limit is a defined logical budget rather than a measurement of allocator overhead. Native support retains tracked allocations until exit; reference execution retains array/map/list values for the same lifetime. Each immutable list node contains a value, next handle and cached length. Cons and length are O(1); tails can be shared, and no mutation operation can create cycles. The reference uses a flat indexed node arena, so dropping a 10000-node chain does not recursively consume the stack. Read-list validates its length, charges `24 * length` once before consuming element tokens, parses all values, then constructs nodes without another charge. This agrees with native failure ordering even when allocation limits and malformed input coincide. The native integer map uses open addressing with a bounded 32768-slot table. It provides expected constant-time operations; worst-case probe behavior is not a guaranteed constant-time bound. The reference interpreter uses a Rust integer map with the same observable lookup/overwrite/capacity rules.

## Native compilation and inspection

`src/language/compute.rs` directly decodes RVB2 and implements validation, canonical encoding, decoded listings and reference execution. `CompiledProgram::execute_with_input` is the reference path; `execute()` supplies empty input. Compilation and inspection never consume input or execute operations.

`src/language/compute_codegen.rs` lowers each instruction PC to a native LLVM basic block, using typed register slots, arithmetic overflow intrinsics, comparisons, loads, stores and direct branches. Register slots and array-construction scratch storage are allocated once in function entry. The backend can optimize these slots; a loop cannot repeatedly grow the native stack through array-construction scratch allocations. A tick call enforces instruction fuel even when LLVM optimizes other operations. Copy becomes typed load/store, nil becomes null, nil-test becomes pointer equality, and assertion becomes a conditional branch to a diagnostic block containing its source PC. Native signed division and remainder have dominating guards for zero and `i64::MIN` with `-1` before any `sdiv`/`srem` executes; neither invalid case can trigger LLVM undefined behavior.

The native executable calls generic C routines for stdin parsing, array/map/node storage, resource checks and output. It does not contain a Rivet decoder, encoded program interpreter, or algorithm-specific solver helper. Integer arithmetic and each algorithm’s comparisons, branches and assertions are emitted LLVM operations. Float literals use exact hexadecimal binary64 constants, integer conversion uses `sitofp`, and arithmetic uses `fadd`/`fdiv` without fast-math. An explicit zero guard dominates Float division. A bitwise exponent check rejects nonfinite arithmetic results before storing them, retaining failures even for unused results. The only new C Float helper is `rv2_return_float(double)`, which checks input completion, defensively validates finiteness and formats output. List traversal and construction occur only when the program’s corresponding records execute; the runtime does not perform digit addition or whole-number conversion. The helper ABI is in [TWO-SUM-PLAN.md](TWO-SUM-PLAN.md), [ADD-TWO-NUMBERS-PLAN.md](ADD-TWO-NUMBERS-PLAN.md), [MEDIAN-PLAN.md](MEDIAN-PLAN.md), and `native/runtime.h`.

Inspection retains the record and payload byte offsets used by the workbench. RVB2 literal convenience-edit metadata is currently absent because a value-producing payload begins with a destination field. Raw hex editing still supports the complete source; source exports remain the actual bytes. Decoding and re-encoding any accepted RVB2 program preserves every byte, including unreachable records and unused declarations.
