# Architecture and decision record

The bootstrap rationale below led to the implemented design specified in `CONTRACT.md` and `LANGUAGE.md`. Both MVPs are complete; validation and current state are recorded separately. Historical alternatives are retained here where they explain the implementation.

## Target

Start on macOS arm64 with Rust 1.90.0 and a browser UI served by the local executable. This target runs on the development machine without a VM, cross compiler, or native app distribution. Keep the core portable where practical; other platforms are unverified until tested.

## Compiler and execution path

Use a small, versioned, typed program representation with an explicit validation stage and a distinct executable intermediate form. Rust owns parsing, validation/lowering, and execution. Define output and HTTP response operations as language/runtime capabilities. Include enough value composition to show computation and verify that program edits propagate through compilation to execution.

The final v1 serialization is JSON, with seven typed operations and console/HTTP entries. The compiler resolves named operands to private typed register indices and emits an inspectable textual disassembly. Execution uses that validated IR. Artifacts are in memory; source is the versioned portable program format. A native backend was deferred because it would add toolchain complexity without helping establish either required demonstration.

## Workbench

Use a local browser workbench for program editing, diagnostics, build results, execution output, and HTTP lifecycle/request inspection. Serve its static assets from the Rust executable. Avoid a frontend build pipeline unless a concrete need justifies it. Keep the frontend independent of example behavior: all executed output and HTTP responses come from compiled programs.

The workbench service and program HTTP service should have separate lifecycles. Bind to loopback, report port conflicts clearly, and stop/join program-server workers deterministically. Document resource and protocol limits. Validate workbench requests and prevent cross-origin mutation of the local runtime.

## Dependencies and performance

Rust 1.90.0 is pinned, and Cargo/npm lockfiles record exact dependencies. serde/serde_json handle representation decoding and JSON output; httparse parses bounded HTTP headers; ctrlc handles shutdown signals. Native HTML/CSS/JS requires no production frontend dependency. Playwright is a development-only test dependency.

For efficiency evaluation, first define a repeatable release-build benchmark on the documented target: compile a fixed program and execute its non-network computation repeatedly; separately measure local HTTP request latency. Record workload, iteration count, timing scope, and build mode. Initial measurements establish a baseline only, with no comparison to mature languages implied.

## Implemented transport decision (2026-10-25)

The initial `tiny_http` transport passed ordinary request/lifecycle tests but independent review reproduced blocking shutdown with an incomplete declared request body. Its unread-body cleanup could also allocate according to an oversized Content-Length. It was replaced before milestone acceptance with an owned Rust TCP listener/stream adapter and `httparse` 1.10.1 for header parsing.

The adapter serves one request per connection, limits headers to 8 KiB/64 fields, bodies to 512 KiB, and request targets to 2048 bytes, rejects ambiguous framing and Transfer-Encoding, imposes a two-second total read deadline, polls cancellation during reads, and bounds socket writes to 250 ms per write. Each listener is owned and dropped by its serving thread; program Stop joins that thread before returning. No unread-body drain happens on drop. Regression tests cover incomplete POST/GET shutdown, immediate rebind, huge/duplicate Content-Length, and request expiry.

Each listener handles requests sequentially. A slow client may delay another request for up to the bounded request deadline. This deliberately small HTTP subset is for local experiments; production concurrency, streaming/chunked requests, TLS, and Internet hosting are outside v1. Standard Content-Length JSON requests from the UI and curl are supported.
