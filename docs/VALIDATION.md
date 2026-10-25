# Validation record

Target: macOS 26.5.2, Apple Silicon (arm64); Rust/Cargo 1.90.0; Node 22.7.0. Browser tests use locked Playwright 1.64.1 and Chromium 156.0.8078.4. Date: 2026-10-25.

## Integrated implementation checks

- `cargo test --locked`: 32 tests pass (24 language, 2 runtime, 3 bounded HTTP transport, 3 workbench API).
- `cargo fmt --check`: passes.
- `cargo clippy --locked --all-targets -- -D warnings`: passes.
- `npm test`: 8 actual Chromium tests pass after the transport replacement. Scenarios include both MVPs, edited output and HTTP response, syntax/reference/overflow diagnostics, repeated requests, missing routes, stop/restart, failed build preserving running service, exact i64 response display, file open/download, keyboard operation, and narrow layout.
- Independent review findings and regression evidence are in [REVIEW-HANDOFF.md](REVIEW-HANDOFF.md). The initial transport's incomplete-request shutdown bug was reproduced and fixed before acceptance.

## Actual interactive UI verification

Performed in the Codex in-app browser against the compiled local workbench on port 7331, using real controls (not mocked API results).

### MVP 1

1. Opened the Hello world example and clicked Build. UI showed build #1 and three instructions.
2. Clicked Run program. Console displayed `Hello, world!` and Completed.
3. Edited the `audience` instruction from `world!` to `engineer!`. UI marked source Modified and disabled Run.
4. Built and ran again. Console displayed `Hello, engineer!` from build #2.
5. Replaced the program with `{"version":1}` and clicked Build. UI displayed three field-specific diagnostics for the missing name, instructions, and entry, and disabled Run.

Screenshot: [Hello world execution](evidence/hello-ui.png).

### MVP 2

Automated UI checks pass; coordinator interactive verification is pending at this checkpoint.

## Clean checkout

Pending at this checkpoint. The final record must identify the tested commit, exact commands, and outcomes.

## Efficiency experiment

The committed `cargo bench --locked --bench baseline` defines a reproducible release-build baseline: 2,000 compilations of the computed example, 20,000 interpreter executions of its compiled artifact, and 500 sequential `/hello` requests with a new loopback TCP connection each time. Wall-clock aggregate divided by iterations; no warm-up, statistical distribution, or cross-language comparison. Timings include allocation/serialization appropriate to each stage. Execution measures value computation, not server startup. Results will be recorded only after both MVPs pass.
