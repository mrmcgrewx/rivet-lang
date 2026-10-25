# Integrated RVB2 review

Reviewed 2026-10-25 against `TWO-SUM-PLAN.md` (`fe95cd0`). The reviewer authored the generic C support, then independently reviewed the compiler, native lowering, source algorithm and coordinator's CLI/API/process integration. This review added only this document; the probes used temporary files outside the repository. No Git mutations were performed.

## Architectural and correctness findings

The 231-byte `examples/two-sum.rvb` contains 21 numeric instruction records and 13 declared typed registers. Its scan, complement subtraction, lookup, equality test, conditional branches, insertion, index increment and result construction are in the binary source. The fixture writer only packs those records. It neither solves inputs nor translates a hidden JSON/text source program.

The algorithm checks the complement before storing the current index, so `[3,3]` correctly returns distinct indices and a one-element array cannot reuse its own entry. Overwriting a previous index for duplicate values still yields a valid pair. The scan returns an empty array when no pair exists, outside LeetCode's exactly-one-answer guarantee. Checked i64 subtraction covers the problem's stated integer range; full-i64 inputs outside that range may correctly fail on arithmetic overflow rather than silently wrap.

The decoder enforces header/body/payload boundaries, exact tags, declared register types, reference and branch-target bounds, Boolean canonicality and exact EOF. Counts are bounded before allocation. Structural/type validation includes unreachable records. The reachable-graph analysis rejects falling off the instruction stream, and its fixed-point intersection correctly checks definitions before operands are read, including the first loop iteration and jumps back to entry. Entry always includes an empty initial path. Constant conditions conservatively retain both successors; termination is enforced dynamically with fuel, not claimed as a static proof.

LLVM lowering emits real native basic blocks, loads/stores, comparisons, checked arithmetic intrinsics and branches. Operands are loaded before an in-place destination is overwritten. Registers and array-construction scratch storage are allocated once at function entry. Given the 64 KiB source bound, scratch storage also remains bounded by the source's reference payloads. Each executed source operation calls the fuel helper; generated return calls output and cleanup. Unreachable tails have valid LLVM terminators and cannot introduce reachable uninitialized reads.

The reference implementation and C support agree on input token rules, array/map observable behavior, lifetime retention, logical allocation charging and fuel count. Maps use generic integer keys/values and caller-provided defaults. Native helpers do not contain the algorithm or an instruction dispatcher. Fixed hash-table capacity is documented; expected lookup behavior is not presented as a guaranteed worst-case constant-time bound. Logical quota accounting is correctly distinguished from actual allocator overhead.

CLI and workbench compile raw `.rvb` bytes without executing input. Runtime data is supplied separately. Workbench input changes reuse the artifact; source edits require a new build. The API allows enough encoded JSON body bytes for the maximum decoded input and then enforces the decoded byte limit. Existing console/server entries reject nonempty runtime input. The process runner drains output and writes input concurrently, closes stdin at the end of delivery, and preserves child stderr/exit status ahead of a broken-pipe delivery error. Timeout/error paths kill and reap before joining their I/O workers.

## Resolved finding

**Artifact metadata still identified compute runtime ABI as version 1.** The initial integrated `src/native.rs` wrote `runtime_abi: 1` for every program, including binaries calling the new `rv2_*` ABI. This did not change execution but made the artifact's inspection metadata misleading. The coordinator changed compute artifacts to `runtime_abi: 2`, retained version 1 for prior entries, and added an assertion to the Two Sum artifact test. The reviewer inspected the correction. No unresolved implementation blocker was found.

## Checks actually performed

| Check | Result |
| --- | --- |
| `cargo build --locked --lib` | Passed; the temporary Rust probes linked the current library |
| `cargo test --locked --test compute_source --test two_sum --test workbench` | 17 tests passed: 10 generic compute, 2 Two Sum/CLI, 5 workbench/API |
| Independent CFG path-state oracle | 10000 generated graphs agreed with compiler acceptance; 659 accepted |
| Independent mutation/truncation probe | 59367 cases, 6534 accepted; all accepted cases re-encoded exactly and exposed valid inspection offsets |
| Generic large-output native probe | Baseline and size profiles returned all 210002 bytes exactly |
| Full unread stdin pipe + fuel failure | Both profiles returned code 1 and the fuel diagnostic in about 155–163 ms, without deadlock |

The CFG oracle generated small graphs with constants, in-place arithmetic, comparisons, array construction, input, jumps, branches and returns. It independently enumerated `(PC, initialized-register-set)` path states and looked for any uninitialized read or reachable fallthrough, rather than copying the compiler's intersecting fixed-point implementation. Seed was `0x8db7125e9a61`; this is bounded model-based evidence, not exhaustive proof of all 256-instruction programs.

The mutation probe tested every truncated prefix and all 256 possible byte replacements at each position of the committed Two Sum source. Counts include unchanged replacements. Accepted sources had byte-exact re-encoding and in-bounds inspection offsets. The probe did not execute arbitrary mutated loops or claim exhaustive native correctness.

The large-output probe directly authored a two-instruction RVB2 `read_array`/`return` program, built it under both native profiles, and fed 10000 INT64_MIN values. It compared native output byte-for-byte with both the expected decimal array and reference execution, proving the process runner no longer truncates at its old 131072-byte capture cap. The blocked-input probe directly authored an unconditional self-jump, supplied 262144 bytes the program never reads, and verified fuel failure while the writer would otherwise be blocked on its pipe. Temporary native artifacts were removed afterward. The coordinator is adding these cases as durable regressions.

The generic C baseline/size and UBSan suites were also run during implementation and passed; details and exact limits are in `TWO-SUM-RUNTIME-HANDOFF.md`. That is implementation validation by the runtime author, not a separate independent audit of their own C contribution.

## Remaining acceptance ownership

No correctness blocker remains from this review. The coordinator remains responsible for the complete final regression run, actual in-app browser interaction, current documentation, clean-checkout reproduction and publication. This review does not claim those later activities occurred. The prototype's fixed limits, single-computation scope, Apple Silicon verification and conventional LLVM/Clang backend remain explicit constraints.
