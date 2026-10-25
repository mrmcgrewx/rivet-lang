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

1. Loaded HTTP server, built it (build #3, two typed instructions), and clicked Start server. The UI showed `http://127.0.0.1:8787` and `/hello`.
2. Sent `/hello` twice using the request inspector. Both responses were HTTP 200 with `application/json; charset=utf-8` and exact body `{"message":"hello world"}`; the displayed request count advanced to 2.
3. Clicked Stop. The UI showed Stopped / Listener released; an independent TCP connect check confirmed port 8787 no longer accepted connections.
4. Changed the program's text to `hello from Rivet`, rebuilt as #4, and restarted on the same port. The inspector returned `{"message":"hello from Rivet"}` with HTTP 200.
5. Stopped again successfully. The workbench remains available independently of the program listener.

Screenshot: [HTTP status, repeated requests, and JSON payload](evidence/http-ui.png).

## Clean checkout

Cloned the public GitHub repository into a new empty temporary directory at commit `ce9e2ec33a08676da85aef2228e658718ea7be4c`. No existing build output or node_modules directory was copied. Cargo compiled dependencies into the checkout's fresh target directory; the machine's Rust installation and downloaded package/browser caches were available. Tracked files remained clean afterward.

Commands actually run successfully:

```sh
git clone https://github.com/mrmcgrewx/rivet-lang.git
cd rivet-lang
git checkout ce9e2ec33a08676da85aef2228e658718ea7be4c
cargo build --locked --release
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
./target/release/rivet check examples/hello.rivet.json
./target/release/rivet run examples/hello.rivet.json
npm ci
npx playwright install chromium
npm test
cargo bench --locked --bench baseline
```

Outcomes: clean release build succeeded; 32 Rust tests passed; 8/8 browser tests passed in 5.7 seconds; CLI printed typed IR and `Hello, world!`; npm reported zero vulnerabilities. Browser tests started the workbench from this clean checkout on port 7332.

Additional release-binary smoke checks launched `./target/release/rivet ui --port 0`, loaded its embedded HTML, built the server example with port 0 through the API, started it, and verified three direct HTTP 200 JSON responses. SIGINT returned exit code 0 and both workbench and program ports refused new connections. A standalone `rivet run` server also returned the expected JSON and shut down with exit code 0 on SIGINT. `rivet check` on an invalid program returned nonzero with a field-specific diagnostic.

GitHub CI passed for the [Hello milestone](https://github.com/mrmcgrewx/rivet-lang/actions/runs/40211875101) and [HTTP milestone](https://github.com/mrmcgrewx/rivet-lang/actions/runs/40212011508). CI runs formatting, strict Clippy, all Rust tests, and real Chromium UI tests. The final completion commit changes only documentation/evidence, not the tested implementation.

## Efficiency experiment

The committed `cargo bench --locked --bench baseline` defines a reproducible release-build baseline: 2,000 compilations of the computed example, 20,000 interpreter executions of its compiled artifact, and 500 sequential `/hello` requests with a new loopback TCP connection each time. Wall-clock aggregate divided by iterations; no warm-up, statistical distribution, or cross-language comparison. Timings include allocation/serialization appropriate to each stage. Execution measures value computation, not server startup. Both MVPs passed before the experiment ran. One clean-checkout release run measured:

| Workload | Mean from aggregate wall time |
| --- | --- |
| Compile computed example | 14,490 ns/program (14.49 µs) |
| Execute compiled computed example | 1,259 ns/execution (1.259 µs) |
| New-connection loopback HTTP request | 2,708 µs/request (2.708 ms) |

These are single-run observations on the target machine, not stable service-level guarantees or comparisons with other languages. HTTP timing includes transport polling, TCP setup, and response handling. The two-millisecond accept poll deliberately trades throughput/latency for a simple bounded local runtime. No optimization expansion was undertaken.
