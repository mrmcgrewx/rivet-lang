# Project state

State version: 14. Updated: 2026-10-25.
Public repository: https://github.com/mrmcgrewx/rivet-lang

## Current objective and implementation

New work in progress: Add Two Numbers as binary source with real linked nodes, reusable copy/division/remainder/assertion operations, and a native digit-by-digit algorithm. `ADD-TWO-NUMBERS-PLAN.md` fixes encoding, semantics, ownership and acceptance before delegation. `ROADMAP.md` records the user's broader goal of an AI-oriented language competitive with established languages, distinguishing implemented features from future capabilities and performance measurements. No new completion is claimed yet.

Two Sum is complete. RVB2 supports typed mutable registers, input, arrays/maps, comparisons, checked subtraction and native control flow. The 231-byte `examples/two-sum.rvb` contains the complete one-pass algorithm. CLI and GUI supply runtime data separately, and one native build handles multiple cases. `TWO-SUM-PLAN.md` fixed the contract before delegation (`fe95cd0`); implementation is published at `9594d6f`. `COMPUTE-FORMAT.md` specifies the bytes and `TWO-SUM.md` explains the algorithm. Integrated tests, review, actual browser interaction and clean public-clone reproduction passed. See `TWO-SUM-VALIDATION.md`.

The user requires actual binary files as authored source. The previous JSON-source design was not the intended experiment; its completion claims are historical. Native compilation remains required. RVB1 and RVB2 `.rvb` records carry numeric opcodes, type tags, register references and payloads. The direct decoders construct typed IR without JSON, then the established LLVM/Clang path produces native executables.

CLI source commands now load binary. The workbench stores byte arrays, receives binary example files, sends raw source to Build/Inspect, exposes decoded and hex views, edits literal/port bytes, and downloads real `.rvb` files. It preserves native artifact/code views and OS-process lifecycle. Legacy JSON parsing remains only a library/test compatibility path; public examples and source APIs are binary.

Read `BINARY-PLAN.md` for committed requirements/interfaces/ownership and `BINARY-FORMAT.md` for every byte of the source encoding. `LANGUAGE.md` gives semantics. Current evidence is `BINARY-VALIDATION.md`; native/interpreter history is retained separately.

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

Commits remain the ledger; no Issues/Projects board. Coordinator owns shared docs, backend integration, examples, Git and final acceptance. Agents own binary codec/spec/tests, binary UI/browser tests, and independent review as specified in the committed plan. All contributions must persist handoffs.

## Evidence and limits

Directly authored source files are 49/64/159 bytes. Core tests cover byte bounds, canonical roundtrip, all current operations, exact scalar data, malformed/random input, native/reference outcomes, binary API fetch/inspect/edit/build, JSON rejection and standalone artifacts. The binary benchmark verifies semantics under both profiles; native size results remain modest (Hello −5.4%, server −0.65%, computed tie) with longer builds.

A binary file is not encryption or intrinsically exclusive to AI. Literal strings may remain visible; GUI/CLI decoding makes instructions readable. The experiment has not measured AI authoring advantages. RVB1 remains straight-line; RVB2 adds branches/loops and runtime computation, with one-million-instruction fuel and 64 MiB logical allocation charging. User functions, persistence and HTTP request-dependent values remain unsupported. HTTP responses are computed at startup. RVB1 per-value limits are not aggregate hostile-program memory quotas; RVB2's logical budget is not an RSS guarantee. Native transport is loopback-only and bounded, not production HTTP. Only macOS Apple Silicon is verified; LLVM/Clang emits native machine code.

## Handoff

The original binary-source correction remains complete at `73f86eda8ba2a5af3fdb1d33acdb783661d32bc3`; Two Sum is complete at implementation checkpoint `9594d6f54c070d01abb3cabd36182bb8f48bb634` plus this final evidence update. No required work remains for the requested solution. Local `.rivet/two-sum-deliverables/two-sum` is a 35384-byte native executable verified with all three published cases. Keep the user's live workbench on port 7331 available; browser state may change while they try examples. Future features should preserve version compatibility and generic runtime boundaries.

Continue from the binary specification and preserve canonical source bytes, not the old JSON frontend. Do not use historical JSON-oriented screenshots or completion records as proof of current acceptance. New language features or measured AI-authoring experiments require their own scoped design and validation.
