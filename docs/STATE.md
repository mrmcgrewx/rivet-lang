# Project state

State version: 5. Updated: 2026-10-25.

Public repository: https://github.com/mrmcgrewx/rivet-lang

| Milestone | Status | Evidence |
| --- | --- | --- |
| Bootstrap | Complete | `efefdd1`, `eb4ef43` pushed before implementation; contracts `e4e6b01`, review ownership `72ca645` |
| MVP 1: standalone native Hello world | Reopened — interpreter prototype at `2730be7` | Typed IR, edited output, invalid-program diagnostics, automated and actual browser verification |
| MVP 2: standalone native HTTP server | Reopened — interpreter prototype at `ce9e2ec` | Actual start/repeated request/stop/edit/restart verified; screenshot captured |
| Native reproducibility and efficiency evidence | Pending | Clean clone of `ce9e2ec` passed setup/build/tests/release smoke; baseline recorded |

## Implemented

Rivet v1 is a typed single-assignment JSON representation compiled to private indexed register instructions and executed by a Rust interpreter. The binary embeds a plain HTML/CSS/JS workbench. Console and HTTP behavior come from program values. Workbench build artifacts and running servers are independent. Networking is loopback-only with bounded request parsing and cancellation-aware owned sockets.

The implementation contract, semantics, tests, and agent handoffs are committed project materials. The coordinator integrated all contributions. GitHub mutations use the established App helpers; credentials and private machine instructions are not stored here. Initial repository creation failed until the owner accepted updated App permissions, after which creation/push succeeded.

## Validation and completion

See `docs/VALIDATION.md` for exact commands, clean-checkout evidence, screenshots, release smoke checks, CI links, and baseline measurements. Final implementation checks: 32 Rust tests, 8 Chromium UI tests, formatting, and strict Clippy all passed. Both actual UI flows include edited program behavior. The clean public clone at `ce9e2ec` was rebuilt and tested without copied build outputs. Final completion updates are documentation/evidence only.

Milestone references:

- Brief/bootstrap: `efefdd1` and `eb4ef43`.
- Language/API/ownership contract: `e4e6b01`; independent review assignment: `72ca645`.
- Implementation and Hello world UI acceptance: `2730be7`.
- HTTP UI acceptance: `ce9e2ec`.
- Final completion/clean-reproduction evidence: the commit containing this state version (see `git log -1`).

The user clarified that native compilation is core to completion. Interpreter-only validation above is historical evidence, not acceptance of the corrected requirements. Required work is defined in `docs/NATIVE-PLAN.md`: direct LLVM lowering, standalone native support/runtime, persistent executable artifacts, OS-process execution in the workbench, reference/native differential checks, and measured native size/build-time comparison. Preserve the existing work while completing these milestones. Request-dependent computation remains optional subsequent language work.

## Material limitations

Straight-line language: no branching, loops, functions, native code generation, persistent storage, or request-dependent computations. HTTP JSON is computed at server startup. Sequential HTTP/1.x subset: one request per connection, no chunked request bodies, TLS, or production hosting. Two-second total request read deadline; loopback only. Editor state is memory-only; download before reload. Browser verification targets Chromium/macOS arm64. Duplicate JSON object keys currently use last-value semantics; duplicate instruction IDs and route paths are rejected.
