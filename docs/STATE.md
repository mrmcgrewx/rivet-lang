# Project state

State version: 8. Updated: 2026-10-25.

Public repository: https://github.com/mrmcgrewx/rivet-lang

## Public documentation

The README now serves as the developer-facing project overview: motivation, a complete language example, setup, standalone console/HTTP programs, workbench, compiler/runtime responsibilities, CLI reference, measured performance, tests, and current scope. Development history remains in the linked project records. Documentation verification checked local links and section anchors, matched the inline program to the committed Hello example, and compiled/ran it with the documented output and inspection files. This change does not alter implementation behavior.

## Current implementation

Rivet validates a typed single-assignment graph serialized in JSON, lowers it to private indexed instructions, emits LLVM directly, and uses Clang to produce standalone native executables with small linked C helpers. Native compilation is core to completion. The interpreter is retained as a correctness reference, explicitly selected by `rivet interpret`. Both Hello and HTTP compile through the same path.

The workbench builds persistent artifacts, launches them as OS children, shows path/download/size/profile/target, exposes typed IR/LLVM/assembly, displays exit status and server PID, and makes real TCP requests. Source edits, accepted build, and running server remain separate. SIGINT/SIGTERM stops managed processes; executables remain on disk.

## Milestones and ledger

| Milestone | Status | Evidence |
| --- | --- | --- |
| Bootstrap and interpreter groundwork | Historical complete | `efefdd1`, `eb4ef43`, contracts `e4e6b01`; interpreter Hello `2730be7`, server `ce9e2ec`, historical record `3d71cde` |
| Revised native contract | Complete | `bddf7f0`; `docs/NATIVE-PLAN.md` |
| Native compiler, support, CLI and artifact execution | Implemented and tested | `279dd92`; compiler/runtime handoffs and native artifact tests |
| Native Hello and HTTP workbench acceptance and review | Complete | 45 Rust and 10 Chromium tests pass, actual Hello and server edit/build/run/request/stop verified; native review finding resolved |
| Clean native reproduction and publication | Complete | Clean public clone `00116f9`: full checks, release/native copies, system-library inspection; final evidence in the commit containing state version 7 |

Use commits as the ledger; no Issues/Projects board. Coordinator owns Git writes through the machine's GitHub App helpers. `NATIVE-PLAN.md` records delegated ownership; agents persisted compiler/runtime/UI/review handoffs. The coordinator completed combined acceptance and public checkpoint publication. `00116f9` contains native UI acceptance and measurements; the final documentation commit contains clean-reproduction evidence.

## Evidence

`docs/NATIVE-VALIDATION.md` is the current acceptance record. `docs/VALIDATION.md` is historical interpreter-only evidence and does not establish native completion. Tests cover exact standalone copies after workbench shutdown with source/Rivet absent, byte-matched executable download, Clang failure/overwrite behavior, all operations and error ordering against reference semantics, exact JSON/i64/Unicode/NUL, both profiles, lifecycle, and UI controls.

Independent review found and verified a SIGTERM child-orphan fix. No unresolved implementation blocker remains. C warnings and UBSan pass; Apple ASan failed to initialize before main and is not claimed. Size benchmark: Hello −5.4%, server −0.65%, computed tie against identical -O0 LLVM/support, with longer builds. No universal performance or AI-generation benefit is claimed.

## Material limits

Straight-line language, without user branches, loops, functions, persistent storage, or request-dependent computation. Responses are computed at startup. Human inspection is supported; literal AI-only unreadability is not promised. No handwritten machine emitter; LLVM/Clang is the backend. Native helpers supply memory/text/JSON/OS support, without a source parser or interpreter.

Only macOS Apple Silicon is verified. Executables use system libraries; distribution signing and cross-compilation are outside this milestone. HTTP is a loopback-only sequential subset with bounded framing/deadlines, without TLS/chunked requests/production hosting. Editor state is memory-only; save source before reload. Artifacts persist but UI history does not. Duplicate JSON keys use last-value semantics. Per-value bounds are not an aggregate hostile-program memory quota. SIGKILL/crashes do not guarantee child cleanup.

## Handoff

No required native milestone work remains. Clean public checkout `00116f95fee0bb6bac236d9d34974825c3131e41` passed formatting, strict Clippy, 45 Rust tests, release build, C baseline/size and UBSan, locked npm setup, and 10 Chromium scenarios. Separate copies of its Hello/server executables ran with cleared environment and nonexistent PATH; file inspection confirmed arm64 Mach-O and only libSystem dynamic linkage. The working checkout also has `.rivet/deliverables/hello` and `server` ready to run, with sidecars. The live workbench at port 7331 was left showing native Hello output; its program server is stopped.

To continue, read `NATIVE-PLAN.md`, `NATIVE-VALIDATION.md`, and `NATIVE-REVIEW.md`; choose a new language feature or a specific measurement objective rather than treating native compilation as pending. Existing historical validation/handoff files remain useful but describe the earlier interpreter prototype.
