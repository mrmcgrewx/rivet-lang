# Project state

State version: 18. Updated: 2026-10-25.
Public repository: https://github.com/mrmcgrewx/rivet-lang

## Current objective and implementation

Median of Two Sorted Arrays is implemented and locally verified. `MEDIAN-PLAN.md` fixed additive Float encoding, logarithmic binary-partition source, speed profile, comparison methodology and ownership at `cccf624` before delegation. The 803-byte/81-operation source passes 275 oracle cases under three native profiles and 3135 exhaustive small reference cases. Eight generic Float tests, existing regressions, C/UBSan, 18 browser scenarios and actual in-app runs pass. The C/Python comparison harness passes its bounded correctness smoke; independent review, a clean committed checkout, full comparative measurements and public CI are final gates. See `MEDIAN-VALIDATION.md` and `MEDIAN-BENCHMARK-METHOD.md`.

Add Two Numbers is complete. Implementation `432cd26b4c4cf4a848d8b41695a54f681c4890c4` includes `examples/add-two-numbers.rvb`: 657 binary bytes and 70 operations over real linked nodes. Reusable typed copy, guarded division/remainder, assertions and list operations extend RVB2 without changing prior files. Native/reference/big-integer-oracle tests, generic list tests, C/UBSan, 16 browser scenarios and actual in-app runs passed. Independent review found no blocker; a fresh public clone passed all 85 Rust tests and 16 browser scenarios, and public CI succeeded. See `ADD-TWO-NUMBERS-VALIDATION.md`. `ADD-TWO-NUMBERS-PLAN.md` fixed the contract at `8a6b220`; `ROADMAP.md` records the user's broader agent-language/performance goal without claiming current competitiveness.

Two Sum is complete. RVB2 supports typed mutable registers, input, arrays/maps, comparisons, checked subtraction and native control flow. The 231-byte `examples/two-sum.rvb` contains the complete one-pass algorithm. CLI and GUI supply runtime data separately, and one native build handles multiple cases. `TWO-SUM-PLAN.md` fixed the contract before delegation (`fe95cd0`); implementation is published at `9594d6f`. `COMPUTE-FORMAT.md` specifies the bytes and `TWO-SUM.md` explains the algorithm. Integrated tests, review, actual browser interaction and clean public-clone reproduction passed. See `TWO-SUM-VALIDATION.md`.

The user requires actual binary files as authored source. The previous JSON-source design was not the intended experiment; its completion claims are historical. Native compilation remains required. RVB1 and RVB2 `.rvb` records carry numeric opcodes, type tags, register references and payloads. The direct decoders construct typed IR without JSON, then the established LLVM/Clang path produces native executables.

CLI source commands now load binary. The workbench stores byte arrays, receives binary example files, sends raw source to Build/Inspect, exposes decoded and hex views, edits literal/port bytes, and downloads real `.rvb` files. It preserves native artifact/code views and OS-process lifecycle. Legacy JSON parsing remains only a library/test compatibility path; public examples and source APIs are binary.

Read `BINARY-PLAN.md`, `TWO-SUM-PLAN.md` and `ADD-TWO-NUMBERS-PLAN.md` for committed contracts, and `BINARY-FORMAT.md` / `COMPUTE-FORMAT.md` for source encoding. `LANGUAGE.md` gives semantics. Current linked-data evidence is `ADD-TWO-NUMBERS-VALIDATION.md`; prior binary, Two Sum and native/interpreter validation is retained separately.

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
| Float / Median | Implemented and locally verified | Binary partition source, reusable finite binary64, three native profiles, oracle/generic tests and 18 browser scenarios |
| Comparative performance | Method implemented, final measurements pending | Equivalent C/Python partition search, process and preloaded scopes, per-child RSS; bounded correctness smoke passes |
| Median clean reproduction | Pending | Fresh committed clone, full measurement and public CI remain |

Commits remain the ledger; no Issues/Projects board. Coordinator owns shared docs, backend integration, examples, Git and final acceptance. Agents own binary codec/spec/tests, binary UI/browser tests, and independent review as specified in the committed plan. All contributions must persist handoffs.

## Evidence and limits

The six directly authored source files are 49/64/159/231/657/803 bytes. Tests cover byte bounds, exact roundtrips, typed data/control flow, aliases/shared tails, checked integer/Float arithmetic, malformed input, native/reference/oracle outcomes, GUI state and independent artifacts. Historical RVB1 size measurements predate the compute runtime extensions; they do not establish current artifact sizes or runtime-speed competitiveness. `ROADMAP.md` defines the broader measurement goals; median comparisons are scoped to one algorithm and host.

A binary file is not encryption or intrinsically exclusive to AI. Literal strings may remain visible; GUI/CLI decoding makes instructions readable. The experiment has not measured AI authoring advantages. RVB1 remains straight-line; RVB2 adds branches/loops and runtime computation, with one-million-instruction fuel and 64 MiB logical allocation charging. User functions, persistence and HTTP request-dependent values remain unsupported. HTTP responses are computed at startup. RVB1 per-value limits are not aggregate hostile-program memory quotas; RVB2's logical budget is not an RSS guarantee. Native transport is loopback-only and bounded, not production HTTP. Only macOS Apple Silicon is verified; LLVM/Clang emits native machine code.

## Handoff

Prior milestones are complete. Finish median's independent review, clean public-checkout/CI gate and comparative report before marking the current milestone complete. `.rivet/median-deliverables/median` is a 34792-byte speed-profile standalone executable; the actual UI's size-profile artifact is 35336 bytes. The live release workbench on port 7331 includes median; actual UI verification ran odd/fractional results, both-empty failure and negative-half recovery using one build. Preserve the user's live session. Future work should follow `ROADMAP.md`, preserve version compatibility and keep algorithm logic out of generic runtime helpers.

Continue from the binary specification and preserve canonical source bytes, not the old JSON frontend. Do not use historical JSON-oriented screenshots or completion records as proof of current acceptance. New language features or measured AI-authoring experiments require their own scoped design and validation.
