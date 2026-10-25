# Project state

State version: 22. Updated: 2026-10-25.
Public repository: https://github.com/mrmcgrewx/rivet-lang

## Current objective and implementation

Native fuel optimization is complete at implementation `c279276de3866f1e918fc8394900abde62208b1c`. `NATIVE-OPTIMIZATION-PLAN.md` fixed ownership and the unchanged comparison contract at `51745ac`. The same counter/check is visible to LLVM, backed by the unchanged C tick and a stable-address getter; all safety limits and failure ordering remain. Clean before/after default benchmarks measured **559.45 → 104.14 ns/query (5.37×)** on balanced 1000 + 1000 inputs, narrowing C's advantage from 16.15× to **3.02×**. All eight families improved 4.56×–5.57×. Balanced standalone peak RSS stayed at 1.406 MiB; the speed executable grew by 16 bytes and median native build time by 22 ms. `NATIVE-OPTIMIZATION-RESULTS.md` preserves scopes, raw reports and remaining gaps. A fresh public checkout passed all 100 Rust tests, strict C/UBSan and 18 browser scenarios; actual UI and independent review passed. Public CI run `40235678882` succeeded. See `NATIVE-OPTIMIZATION-VALIDATION.md`.

Median of Two Sorted Arrays is complete at implementation `b72a8bd569f217ce9ce31358c261fe0e099b94f5`. `MEDIAN-PLAN.md` fixed additive Float encoding, logarithmic binary-partition source, speed profile, comparison methodology and ownership at `cccf624` before delegation. The 803-byte/81-operation source passes 275 oracle cases under three native profiles and 3135 exhaustive small reference cases. A fresh public clone passed all 96 Rust tests, C/UBSan, 18 browser scenarios and the benchmark correctness smoke; actual in-app runs and independent review passed. Public CI run `40233598578` succeeded. See `MEDIAN-VALIDATION.md` and `MEDIAN-REVIEW.md`.

The full C/Python comparison ran from that clean checkout with seven samples across eight families/128 cases. For balanced 1000 + 1000 inputs, median preloaded times were Rivet 558.91 ns/query, C 34.19 ns and Python 1439.22 ns; ordinary-process peak RSS medians were 1.391/1.328/15.125 MiB. Different scopes are intentionally separate. `MEDIAN-BENCHMARK-RESULTS.md`, the method and raw JSON preserve all data and limits. Rivet was 2.58× faster than Python and 16.35× slower than C in that preloaded family; no broader competitiveness or AI-authoring benefit is established.

Add Two Numbers is complete. Implementation `432cd26b4c4cf4a848d8b41695a54f681c4890c4` includes `examples/add-two-numbers.rvb`: 657 binary bytes and 70 operations over real linked nodes. Reusable typed copy, guarded division/remainder, assertions and list operations extend RVB2 without changing prior files. Native/reference/big-integer-oracle tests, generic list tests, C/UBSan, 16 browser scenarios and actual in-app runs passed. Independent review found no blocker; a fresh public clone passed all 85 Rust tests and 16 browser scenarios, and public CI succeeded. See `ADD-TWO-NUMBERS-VALIDATION.md`. `ADD-TWO-NUMBERS-PLAN.md` fixed the contract at `8a6b220`; `ROADMAP.md` records the user's broader agent-language/performance goal without claiming current competitiveness.

Two Sum is complete. RVB2 supports typed mutable registers, input, arrays/maps, comparisons, checked subtraction and native control flow. The 231-byte `examples/two-sum.rvb` contains the complete one-pass algorithm. CLI and GUI supply runtime data separately, and one native build handles multiple cases. `TWO-SUM-PLAN.md` fixed the contract before delegation (`fe95cd0`); implementation is published at `9594d6f`. `COMPUTE-FORMAT.md` specifies the bytes and `TWO-SUM.md` explains the algorithm. Integrated tests, review, actual browser interaction and clean public-clone reproduction passed. See `TWO-SUM-VALIDATION.md`.

The user requires actual binary files as authored source. The previous JSON-source design was not the intended experiment; its completion claims are historical. Native compilation remains required. RVB1 and RVB2 `.rvb` records carry numeric opcodes, type tags, register references and payloads. The direct decoders construct typed IR without JSON, then the established LLVM/Clang path produces native executables.

CLI source commands now load binary. The workbench stores byte arrays, receives binary example files, sends raw source to Build/Inspect, exposes decoded and hex views, edits literal/port bytes, and downloads real `.rvb` files. It preserves native artifact/code views and OS-process lifecycle. Legacy JSON parsing remains only a library/test compatibility path; public examples and source APIs are binary.

Read `BINARY-PLAN.md`, `TWO-SUM-PLAN.md`, `ADD-TWO-NUMBERS-PLAN.md`, `MEDIAN-PLAN.md` and `NATIVE-OPTIMIZATION-PLAN.md` for committed contracts, and `BINARY-FORMAT.md` / `COMPUTE-FORMAT.md` for source encoding. `LANGUAGE.md` gives semantics. Current optimization evidence is `NATIVE-OPTIMIZATION-VALIDATION.md` and `NATIVE-OPTIMIZATION-RESULTS.md`; prior milestones retain their historical validation.

## Milestones

| Milestone | Status | Evidence |
| --- | --- | --- |
| Interpreter groundwork | Historical | `2730be7`, `ce9e2ec`, `3d71cde` |
| Native backend/artifact foundation | Complete, reused | `279dd92`, UI `00116f9`, clean evidence `0bedfba` |
| Binary source contract | Complete | `372d143`, committed before delegation |
| Binary decoder, CLI, source files and API | Complete | Direct binary sources; 59 integrated Rust tests, strict Clippy and release build passed |
| Binary GUI and independent review | Verified | 12 Chromium scenarios passed; independent mutation/differential/race probes passed; hex gutter finding resolved |
| Actual binary UI | Verified | Binary Hello/hex/decoded edit/native run and binary HTTP port edit/build/request/stop captured |
| Clean public reproduction | Complete | Fresh clone `73f86ed`: byte-exact fixture regeneration, 59 Rust and 12 browser tests, C/UBSan, independent artifacts, successful CI |
| RVB2 / Two Sum implementation | Complete | `9594d6f`: 21 binary operations; native/reference/oracle comparisons, generic compute tests, 14 browser scenarios and actual in-app runs passed |
| RVB2 clean reproduction | Complete | Fresh public clone `9594d6f`: 73 Rust tests, 14 browser scenarios, C/UBSan, release build, exact fixture regeneration and independent native copies passed; GitHub CI run `40228140611` succeeded |
| Linked-data / Add Two Numbers | Complete | `432cd26`: 657-byte program, real nodes, reusable copy/div/rem/assert, 270 oracle cases under both native profiles, independent review, 16 browser scenarios and actual in-app runs |
| Linked-data clean reproduction | Complete | Fresh public clone `432cd26`: 85 Rust tests, 16 browser scenarios, C/UBSan, release build, exact fixture regeneration and independent native copies passed; GitHub CI run `40230249273` succeeded |
| Float / Median | Complete | `b72a8bd`: binary partition source, reusable finite binary64, three native profiles, oracle/generic tests, independent review and actual UI |
| Comparative performance | Complete for median | Equivalent C/Python search, process and preloaded scopes, per-child RSS; 128 cases and 552 raw observations from clean `b72a8bd` |
| Median clean reproduction | Complete | Fresh public clone: 96 Rust tests, 18 browser scenarios, C/UBSan, release build, exact fixtures and benchmark smoke; CI `40233598578` succeeded |
| Safe native fuel optimization | Complete | `c279276`: exact shared-counter checks exposed to LLVM; 612 independent paired native probe launches; fresh clone passed 100 Rust tests, C/UBSan and 18 browser scenarios; CI `40235678882` succeeded |
| Unchanged before/after comparison | Complete | Clean `b72a8bd` and `c279276`, 128 cases/552 observations each; 5.37× balanced search improvement, 3.02× remaining C gap; all safety/source/benchmark contracts retained |

Commits remain the ledger; no Issues/Projects board. Coordinator owns shared docs, backend integration, examples, Git and final acceptance. Agents own binary codec/spec/tests, binary UI/browser tests, and independent review as specified in the committed plan. All contributions must persist handoffs.

## Evidence and limits

The six directly authored source files are 49/64/159/231/657/803 bytes. Tests cover byte bounds, exact roundtrips, typed data/control flow, aliases/shared tails, checked integer/Float arithmetic, malformed input, native/reference/oracle outcomes, GUI state and independent artifacts. Historical RVB1 size measurements predate the compute runtime extensions; they do not establish current artifact sizes or runtime-speed competitiveness. `ROADMAP.md` defines the broader measurement goals; median comparisons are scoped to one algorithm and host.

A binary file is not encryption or intrinsically exclusive to AI. Literal strings may remain visible; GUI/CLI decoding makes instructions readable. The experiment has not measured AI authoring advantages. RVB1 remains straight-line; RVB2 adds branches/loops and runtime computation, with one-million-instruction fuel and 64 MiB logical allocation charging. User functions, persistence and HTTP request-dependent values remain unsupported. HTTP responses are computed at startup. RVB1 per-value limits are not aggregate hostile-program memory quotas; RVB2's logical budget is not an RSS guarantee. Native transport is loopback-only and bounded, not production HTTP. Only macOS Apple Silicon is verified; LLVM/Clang emits native machine code.

## Handoff

The native fuel optimization and prior milestones are complete; no required acceptance work remains. `.rivet/optimization-deliverables/median` is the updated 34808-byte speed executable; the live workbench on port 7331 built a 35352-byte size executable and returned 2.5 from the unchanged binary source. Earlier artifacts remain available. Preserve the user's live session. Future work should follow `ROADMAP.md`, preserve version compatibility and keep algorithm logic out of generic runtime helpers. Checked array access is the next measured candidate, not an approved removal of bounds checks or a claim of C parity. Reuse the same benchmark and retain exact fuel/failure semantics.

Continue from the binary specification and preserve canonical source bytes, not the old JSON frontend. Do not use historical JSON-oriented screenshots or completion records as proof of current acceptance. New language features or measured AI-authoring experiments require their own scoped design and validation.
