# Project state

State version: 6. Updated: 2026-10-25.

Public repository: https://github.com/mrmcgrewx/rivet-lang

## Current implementation

Rivet validates a typed single-assignment graph serialized in JSON, lowers it to private indexed instructions, emits LLVM directly, and uses Clang to produce standalone native executables with small linked C helpers. Native compilation is core to completion. The interpreter is retained as a correctness reference, explicitly selected by `rivet interpret`. Both Hello and HTTP compile through the same path.

The workbench builds persistent artifacts, launches them as OS children, shows path/download/size/profile/target, exposes typed IR/LLVM/assembly, displays exit status and server PID, and makes real TCP requests. Source edits, accepted build, and running server remain separate. SIGINT/SIGTERM stops managed processes; executables remain on disk.

## Milestones and ledger

| Milestone | Status | Evidence |
| --- | --- | --- |
| Bootstrap and interpreter groundwork | Historical complete | `efefdd1`, `eb4ef43`, contracts `e4e6b01`; interpreter Hello `2730be7`, server `ce9e2ec`, historical record `3d71cde` |
| Revised native contract | Complete | `bddf7f0`; `docs/NATIVE-PLAN.md` |
| Native compiler, support, CLI and artifact execution | Implemented and tested | `279dd92`; compiler/runtime handoffs and native artifact tests |
| Native workbench acceptance and review | Verified; clean reproduction pending | 45 Rust and 10 Chromium tests pass, actual Hello and server edit/build/run/request/stop verified; native review finding resolved |
| Clean native reproduction and publication | Pending final clean clone | Native benchmark and current docs written; clean clone follows the integration checkpoint |

Use commits as the ledger; no Issues/Projects board. Coordinator owns Git writes through the machine's GitHub App helpers. `NATIVE-PLAN.md` records delegated ownership; agents persisted compiler/runtime/UI/review handoffs. The coordinator is responsible for final combined acceptance and publication.

## Evidence

`docs/NATIVE-VALIDATION.md` is the current acceptance record. `docs/VALIDATION.md` is historical interpreter-only evidence and does not establish native completion. Tests cover exact standalone copies after workbench shutdown with source/Rivet absent, byte-matched executable download, Clang failure/overwrite behavior, all operations and error ordering against reference semantics, exact JSON/i64/Unicode/NUL, both profiles, lifecycle, and UI controls.

Independent review found and verified a SIGTERM child-orphan fix. No unresolved implementation blocker remains. C warnings and UBSan pass; Apple ASan failed to initialize before main and is not claimed. Size benchmark: Hello −5.4%, server −0.65%, computed tie against identical -O0 LLVM/support, with longer builds. No universal performance or AI-generation benefit is claimed.

## Material limits

Straight-line language, without user branches, loops, functions, persistent storage, or request-dependent computation. Responses are computed at startup. Human inspection is supported; literal AI-only unreadability is not promised. No handwritten machine emitter; LLVM/Clang is the backend. Native helpers supply memory/text/JSON/OS support, without a source parser or interpreter.

Only macOS Apple Silicon is verified. Executables use system libraries; distribution signing and cross-compilation are outside this milestone. HTTP is a loopback-only sequential subset with bounded framing/deadlines, without TLS/chunked requests/production hosting. Editor state is memory-only; save source before reload. Artifacts persist but UI history does not. Duplicate JSON keys use last-value semantics. Per-value bounds are not an aggregate hostile-program memory quota. SIGKILL/crashes do not guarantee child cleanup.

## Next action

Commit/push integrated work and reproduce from that public checkpoint without copied outputs, then update this state and the validation record with final evidence and commit references. Do not call the corrected milestones complete before those steps pass.
