# Add Two Numbers — reusable linked-list capabilities

Implement [LeetCode Add Two Numbers](https://leetcode.com/problems/add-two-numbers/description/) as actual binary Rivet source, preserving all existing examples. Two nonempty reverse-order digit lists represent nonnegative integers; return their sum as a list. Inputs may have 100 digits, so the program must perform node-by-node addition with carry rather than converting whole numbers to i64. The solution must compile to native code and run on different inputs without rebuilding. Linked lists must be actual node references, not an array-only simulation hidden behind a solver helper.

The user explicitly wants these exercises to grow Rivet into a practical language for AI agents, with eventual performance competitive with established languages. Every new operation below is general-purpose and requires tests beyond this algorithm. Binary representation alone is not a performance claim; `ROADMAP.md` records the broader capability and measurement work. This milestone implements linked-list computation, not the whole long-term roadmap.

## Additive RVB2 encoding contract

Keep the RVB2 header, flags, register bank, record layout, entry PC and limits. Old RVB1/RVB2 sources remain byte-exact and keep their semantics. Add type tag 7, `IntList`: a nullable handle to an immutable singly linked node containing an i64 and another IntList. Nil is a valid initialized value, distinct from an uninitialized register. Cons creates a new node sharing an existing tail; no mutation operation exists, so valid programs cannot create cycles. A cached node-chain length makes length checks and cons O(1).

All register fields below are u16. Value-producing payloads start with the destination. Copy's result tag must equal both registers' declared type; all current compute types are supported. Void result tag is zero.

| Opcode | Result | Payload / behavior |
| --- | --- | --- |
| 16 | destination type | destination, source; copy scalar or handle, preserving map aliasing |
| 17 | Int (2) | destination, left Int, right Int; signed division truncating toward zero |
| 18 | Int (2) | destination, left Int, right Int; signed remainder with dividend's sign |
| 53 | void | IntList; consume remaining input whitespace, print node values as a compact array plus newline, return success |
| 54 | void | Bool; fail execution if false with `assertion failed` |
| 60 | IntList (7) | destination; nil handle |
| 61 | IntList (7) | destination, Int value, IntList tail; allocate immutable node |
| 62 | Bool (3) | destination, IntList; test nil |
| 63 | Int (2) | destination, IntList; read head value; nil fails |
| 64 | IntList (7) | destination, IntList; read next handle; nil fails |
| 65 | IntList (7) | destination; read count-prefixed list from stdin |
| 66 | Int (2) | destination, IntList; length, including zero for nil |

Division and remainder fail on zero divisor and on `i64::MIN / -1` or `i64::MIN % -1`; guards must execute before LLVM sdiv/srem to avoid undefined behavior. Checked arithmetic, assertions and list operations retain ordered runtime failure semantics. CFG validation must account for all reads, destinations and both return terminators; nil dereference remains a runtime error, not a compile-time execution step.

## Runtime representation and ABI

Keep the existing compute ABI/version 2 and add symbols; original signatures/behavior remain unchanged. Lower nil as LLVM null, is-nil as pointer equality, copy as typed load/store, assertions as direct conditional branches to rv_fail, and division/remainder as guarded native LLVM arithmetic. No arithmetic or algorithm solver helper is added.

```c
void *rv2_read_list(void);
void *rv2_list_cons(int64_t value, void *tail);
int64_t rv2_list_value(void *list);
void *rv2_list_next(void *list);
int64_t rv2_list_len(void *list);
void rv2_return_list(void *list);
```

Native nil is NULL; other handles point to tagged tracked allocations holding value, next pointer and cached length. Generic read_list consumes a length in 0..10000 and that many signed i64 tokens, building nodes in input order. It must not validate decimal-digit semantics or perform addition. Return serializes the supplied chain, checks unconsumed input before output, and leaves generated code responsible for cleanup/return. Nil serializes as `[]`. Existing 256 KiB stdin/output and one-million-instruction fuel limits remain.

Each newly allocated node charges 24 logical bytes against the same cumulative 64 MiB allocation limit. A list chain has at most 10000 nodes; cons onto a full chain fails before allocation. Shared tails are charged only when nodes are created. Reference execution should use an indexed node arena (or equivalent nonrecursive lifetime management) to avoid recursive destruction at depth 10000. Nodes live until execution ends. Header/allocator overhead is outside the logical charge as with existing values. Read-list charging and partial-input failure order must agree across native/reference paths.

## Program and workbench

The authored program reads two lists, checks lengths 1..100 and digit values 0..9, and rejects a final zero digit on multi-node inputs (the problem's no-leading-zero rule). It walks both lists and carry until exhausted, computes digit=sum%10 and carry=sum/10, conses digits onto a temporary reversed result, then reverses that chain in a second binary-encoded loop and returns it. Both traversals and all addition/carry decisions live in `.rvb` and emitted blocks. This is O(max(m,n)) time and O(max(m,n)) allocated nodes; immutable construction uses a temporary chain in addition to the returned chain.

Example metadata adds `{id:"add-two-numbers",name:"Add Two Numbers",url:"/api/examples/add-two-numbers",filename:"add-two-numbers.rvb",input:"3 2 4 3 3 5 6 4\n"}`. Each list is count-prefixed: that example means [2,4,3] plus [5,6,4], output [7,0,8]. Input remains runtime data, never source. Existing build/run APIs and `kind:compute` remain unchanged. GUI help should explain both list lengths and least-significant-digit-first ordering, retaining Two Sum's help and general input guidance.

## Ownership committed before delegation

- Compiler owner: `src/language/compute.rs`, `compute_codegen.rs`, `src/language.rs` only if required, `tests/linked_lists.rs`, `docs/COMPUTE-FORMAT.md`, `docs/ADD-TWO-NUMBERS-COMPILER-HANDOFF.md`. Extend direct decoder/roundtrip/disassembly/CFG/reference/native lowering, test generic copy aliases, lists and div/rem/assert semantics and malformed encodings. No C/UI/integration/shared-doc/Git edits.
- Runtime owner: `native/runtime.c/.h`, `tests/native_runtime/`, `docs/ADD-TWO-NUMBERS-RUNTIME-HANDOFF.md`. Implement generic node ABI and strict C/UBSan tests, including shared tails, nil errors, deep lists, allocation/length/input/output bounds. No solver helper, Rust/UI/shared-doc/Git edits.
- UI owner: `web/`, `tests/ui/`, `docs/ADD-TWO-NUMBERS-UI-HANDOFF.md`. Add explanatory example help and browser tests for all three cases, input-only reuse, long carry and errors without changing source bytes. No Rust/C/shared-doc/Git edits.
- Coordinator: example byte authoring, workbench metadata/API tests, algorithm oracle/native/CLI tests, shared docs/roadmap/state, review, all commits and final acceptance. Owners persist tested findings and handoffs before ending.

## Acceptance

All three published cases, asymmetric lengths, carry chains, zero, no final carry, 100-digit inputs and deterministic randomized valid lists must match an independent arbitrary-precision oracle and the reference interpreter under both native profiles. Reuse each executable; copy it alone into an empty directory. Invalid digits, empty/oversized/noncanonical lists and malformed stdin must fail through source assertions or generic input checks. Tests must prove real list traversal/creation, shared-tail semantics and native checked arithmetic beyond the demonstration. Preserve all existing examples, full regression tests, source roundtrips and GUI behavior. Run strict formatting/Clippy/C/UBSan, actual in-app UI, clean-clone verification and public CI before final completion. No LeetCode submission is requested.
