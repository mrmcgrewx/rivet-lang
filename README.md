# Rivet

A small, experimental typed dataflow language with a Rust compiler/interpreter and a local browser workbench. Programs compile to indexed register instructions; their values determine both console output and HTTP responses.

**In development:** implementation and acceptance verification are underway. See [project state](docs/STATE.md) for completed milestones and evidence.

## Run locally

The primary target is **macOS on Apple Silicon**. Install Rust through [rustup](https://rustup.rs/) if it is not already installed. The repository pins Rust 1.90.0; rustup installs that toolchain on first use.

```sh
git clone https://github.com/mrmcgrewx/rivet-lang.git
cd rivet-lang
cargo run --locked -- ui
```

Open **http://127.0.0.1:7331** in a browser. No Node.js or frontend build is needed to use the workbench. For a release build:

```sh
cargo build --release --locked
./target/release/rivet ui
```

If port 7331 is occupied, use `cargo run --locked -- ui --port 0` and open the address printed by the process. Ctrl-C stops the workbench and its program server.

## The two demonstrations

1. Load **Hello world**, inspect or edit the program, choose **Build**, then run it. The output is `Hello, world!`. Change its text and build/run again to verify that the program controls the result. The workbench also provides diagnostics and the compiled register IR.
2. Load **HTTP server**, build and start it. By default the program binds to `127.0.0.1:8787`; its port is part of the editable program. Send `/hello` from the workbench request inspector, or run:

   ```sh
   curl -i http://127.0.0.1:8787/hello
   ```

   The example returns HTTP 200, `Content-Type: application/json; charset=utf-8`, and `{"message":"hello world"}`. Repeat requests, stop the server, edit the program, and rebuild/restart. Use port `0` in the program to select an available port automatically.

Program edits do not alter a running server. A running server retains its build until explicitly stopped. Invalid builds invalidate the current runnable artifact while preserving an existing server so that Stop continues to work.

## Command line

```sh
cargo run --locked -- check examples/hello.rivet.json
cargo run --locked -- run examples/hello.rivet.json
cargo run --locked -- run examples/server.rivet.json
cargo run --locked -- run examples/computed.rivet.json
```

`check` validates and prints the register IR without executing. `run` prints console output or starts the program server until Ctrl-C. Failures return a nonzero exit code.

## Validation

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

Browser acceptance tests additionally need Node.js 22 or newer:

```sh
npm ci
npx playwright install chromium
npm test
```

The browser tests launch the local workbench automatically. See [validation evidence](docs/VALIDATION.md) for the actual test outcomes and clean-checkout procedure once verified.

## Language and architecture

Rivet v1 serializes custom typed, single-assignment instructions as JSON. `text`, `int`, `bool`, `concat`, checked `add`, `to_text`, and `object` produce typed values. Named references must refer to earlier instructions. The compiler resolves them to register indices and validates the selected console or HTTP entry. The interpreter executes the resulting instructions once in order. Runtime HTTP support handles transport; it serves the route/status/JSON computed by the program.

- [Language semantics](docs/LANGUAGE.md)
- [Implementation contract and API](docs/CONTRACT.md)
- [Architecture decisions](docs/ARCHITECTURE.md)
- [Project brief and acceptance criteria](docs/BRIEF.md)
- [Milestones and continuation state](docs/STATE.md)

Rust dependencies and their transitive versions are locked in `Cargo.lock`. `serde` and `serde_json` provide strict parsing and JSON serialization, `httparse` parses bounded HTTP request headers while Rust owns the loopback sockets, and `ctrlc` provides shutdown signals. The UI uses native HTML/CSS/JavaScript without external assets or a build step. Playwright is only a development/test dependency.

## Scope and limits

This is a straight-line experimental language, without loops, functions, branching, native code generation, persistence, or a package system. HTTP programs compute their responses at start time; request-dependent computation is future work. Both listeners bind to IPv4 loopback. The workbench is intended for one local user and retains one build and at most one program server in memory; downloaded program files preserve source, not live server state. Restarting the workbench clears its build history.

The HTTP runtime is intentionally small: exact GET routes, JSON responses, and no TLS, authentication, streaming, or production deployment support. See the language contract for source, instruction, output, and route limits. The workbench checks Host/Origin and requires a custom JSON-request header for mutations; it does not grant cross-origin API access.

`cargo bench --locked --bench baseline` measures compilation, interpreter execution, and sequential local HTTP requests using the committed examples. It is a baseline experiment, not a claim of superiority to another language. Timing scope and actual results belong in the validation record.
