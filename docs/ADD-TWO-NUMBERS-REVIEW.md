# Independent linked-list milestone review

Reviewed 2026-10-25 against `ADD-TWO-NUMBERS-PLAN.md` and implementation checkpoint `432cd26`. The reviewer implemented the UI, but did not implement the compiler, native runtime or binary algorithm reviewed here. This review changed only this document; independent probe files and artifacts were created outside the repository. No Git mutations were made.

## Result

No unresolved correctness or acceptance blocker was found. The reviewed extension uses actual immutable linked nodes, preserves typed-copy semantics, checks signed division/remainder before potentially undefined native arithmetic, and keeps the application algorithm in binary source and generated native control flow.

## Code findings

- The decoder validates the new result tags, exact payload lengths, declared register types and references. Copy requires both registers to match its result type, including arrays, maps and lists. New operations participate in destination/read tracking; both return forms terminate the CFG. Nil is an initialized list value, while reading an uninitialized list register is rejected.
- LLVM copies load before storing, including in-place updates. Division and remainder both have dominating branches for zero and `i64::MIN` with divisor `-1`; neither invalid case reaches `sdiv`/`srem`. Assertions branch to a failure block that identifies the source PC. Runtime helpers perform generic input/storage/output; arithmetic and algorithm decisions remain generated operations.
- Reference lists use an indexed arena. Native nodes contain an actual tail pointer, value and cached length in aligned, tagged allocations. Copies share existing handles; map copies retain mutable aliasing. Cons creates a new node pointing to an already existing tail, so available operations cannot create cycles. Reference destruction and native allocation-list cleanup are iterative, avoiding recursion and double frees through shared tails.
- Cons checks chain length before charging its new node. Read-list validates length, charges all nodes before parsing element tokens, then constructs the immutable chain in input order without a second charge. Reference and C paths use the same order. Nil reads fail, nil length is zero, and output checks remaining input before writing. Quotas are logical allocation charges, not an RSS promise.
- The 657-byte, 70-operation example reads two generic lists, checks problem constraints, traverses nodes, computes digits/carry and reverses its temporary result through Rivet operations. The writer resolves labels to numeric PCs; names do not enter the source file. The C runtime contains no digit validator, whole-number conversion or addition solver. The coordinator's arbitrary-precision test oracle is separate from the node/carry algorithm and uses exact integer arithmetic.

## Independent checks performed

| Probe | Result |
| --- | --- |
| Initialization/control flow | **10000 generated CFGs** agreed with a separate exhaustive `(PC, initialized-register-set)` path explorer; **1590 accepted**. Generated operations included copies across all current types, list operations, arithmetic, assertions, branches, loops and both return forms. The oracle explores both branch edges rather than reproducing the compiler's fixed-point intersection implementation. |
| Binary robustness | **657 truncated prefixes** and **50000 seeded one-/two-byte mutations** completed without panic. **2979 accepted mutation cases** re-encoded exactly, produced valid inspection metadata and emitted LLVM. Accepted cases are not claimed to be unique. |
| Generic native semantics | A separately byte-authored list transformer emitted each input integer's signed quotient/remainder and retained the original shared handle to report its length. **36 cases** matched Python integer arithmetic and reference execution; **72 native runs** passed across baseline and size profiles. Cases included both signs, empty lists, i64 extrema, destination reuse, scalar/list copies, traversal, construction, reversal and guarded arithmetic failures. |
| C/runtime sanitization | Strict-warning Clang `-O2 -fsanitize=undefined -fno-sanitize-recover=all` compiled the C runtime linked with that generated LLVM. **Four probes passed**, including **4999 input nodes transformed into 9999 output nodes**, signed boundaries, zero divisor and MIN/-1 failure paths. No UBSan diagnostic occurred. |

The CFG/mutation generator used seed `0x63797470655f7265`; randomized list arithmetic used seed `38762026`. These were bounded independent experiments, not exhaustive proof. Temporary probe sources and executables were removed afterward. Existing owner tests, the 16-scenario UI suite, the coordinator's full regression/algorithm oracle run, actual in-app acceptance and clean-clone verification are separate evidence and are not counted as independently rerun here.

The reviewed scope remains immutable i64 lists, bounded resources and the current native target. Broader language capabilities or performance superiority are not established by this review.
