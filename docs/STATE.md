# Project state

State version: 10. Updated: 2026-10-25.
Public repository: https://github.com/mrmcgrewx/rivet-lang

## Current objective and implementation

The user requires actual binary files as authored source. The previous JSON-source design was not the intended experiment; its completion claims are historical. Native compilation remains required. Current source is RVB1 `.rvb` binary records with numeric opcodes, type tags, positional register references and literal/entry payloads. The direct decoder constructs typed IR without JSON, then the established LLVM/Clang path produces native executables.

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

Commits remain the ledger; no Issues/Projects board. Coordinator owns shared docs, backend integration, examples, Git and final acceptance. Agents own binary codec/spec/tests, binary UI/browser tests, and independent review as specified in the committed plan. All contributions must persist handoffs.

## Evidence and limits

Directly authored source files are 49/64/159 bytes. Core tests cover byte bounds, canonical roundtrip, all current operations, exact scalar data, malformed/random input, native/reference outcomes, binary API fetch/inspect/edit/build, JSON rejection and standalone artifacts. The binary benchmark verifies semantics under both profiles; native size results remain modest (Hello −5.4%, server −0.65%, computed tie) with longer builds.

A binary file is not encryption or intrinsically exclusive to AI. Literal strings may remain visible; GUI/CLI decoding makes instructions readable. The experiment has not measured AI authoring advantages. The language remains straight-line, without user branches, loops, functions, persistence or request-dependent values. HTTP responses are computed at startup. Per-value limits are not aggregate hostile-program memory quotas. Native transport is loopback-only and bounded, not production HTTP. Only macOS Apple Silicon is verified; LLVM/Clang emits native machine code.

## Handoff

No required binary-source correction work remains. The complete current evidence is `BINARY-VALIDATION.md`; public implementation checkpoint is `73f86eda8ba2a5af3fdb1d33acdb783661d32bc3`, and the final documentation commit records clean reproduction. Local `.rivet/binary-deliverables/hello` and `server` were built from the binary examples. The live workbench on port 7331 shows Hello bytes and native output, with its server stopped.

Continue from the binary specification and preserve canonical source bytes, not the old JSON frontend. Do not use historical JSON-oriented screenshots or completion records as proof of current acceptance. New language features or measured AI-authoring experiments require their own scoped design and validation.
