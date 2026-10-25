# Project state

State version: 3. Updated: 2026-10-25.

Public repository: https://github.com/mrmcgrewx/rivet-lang

| Milestone | Status | Evidence |
| --- | --- | --- |
| Bootstrap | Complete | `efefdd1`, `eb4ef43` pushed before implementation; contracts `e4e6b01`, review ownership `72ca645` |
| MVP 1: Hello world UI | Complete in this checkpoint | Typed IR, edited output, invalid-program diagnostics, automated and actual browser verification |
| MVP 2: local HTTP server UI | Implemented, automated tests pass | Final coordinator interactive check pending |
| Reproducibility and final handoff | In progress | Clean checkout, benchmark baseline, final documentation/push pending |

## Implemented

Rivet v1 is a typed single-assignment JSON representation compiled to private indexed register instructions and executed by a Rust interpreter. The binary embeds a plain HTML/CSS/JS workbench. Console and HTTP behavior come from program values. Workbench build artifacts and running servers are independent. Networking is loopback-only with bounded request parsing and cancellation-aware owned sockets.

The implementation contract, semantics, tests, and agent handoffs are committed project materials. The coordinator integrated all contributions. GitHub mutations use the established App helpers; credentials and private machine instructions are not stored here. Initial repository creation failed until the owner accepted updated App permissions, after which creation/push succeeded.

## Validation and remaining work

See `docs/VALIDATION.md` for commands and current evidence, and `docs/REVIEW-HANDOFF.md` for resolved review findings. Current integrated checks: 32 Rust tests, 8 Chromium UI tests, formatting, and strict Clippy. Actual Hello UI verification includes edited output and invalid programs.

1. Complete actual HTTP UI start/request/repeat/stop/restart verification and capture evidence.
2. Push milestone acceptance, clone the public repository into a clean temporary checkout, and run documented setup/build/test instructions there.
3. Record a measured efficiency baseline without comparative performance claims.
4. Finalize README/validation/state with milestone references and clean-checkout outcomes; push and verify final remote commit and CI.

## Material limitations

Straight-line language: no branching, loops, functions, native code generation, persistent storage, or request-dependent computations. HTTP JSON is computed at server startup. Sequential HTTP/1.x subset: one request per connection, no chunked request bodies, TLS, or production hosting. Two-second total request read deadline; loopback only. Editor state is memory-only; download before reload. Browser verification targets Chromium/macOS arm64. Duplicate JSON object keys currently use last-value semantics; duplicate instruction IDs and route paths are rejected.
