# Rivet

An experimental typed dataflow language that compiles to standalone native executables, with a local browser workbench. Rust validates the program and emits LLVM IR; Clang produces machine code and links small C helpers. The Rust interpreter remains a reference for correctness tests.

Native compilation is part of completion. The original interpreter-only milestones did not meet that requirement; [the corrected contract](docs/NATIVE-PLAN.md) and [native validation](docs/NATIVE-VALIDATION.md) record the change.

## Run locally

The verified target is **macOS on Apple Silicon**. Install Rust through [rustup](https://rustup.rs/) and Apple's Xcode command-line tools (`xcode-select --install`) if needed. Rust 1.90.0 is pinned; native builds require Clang on PATH. Verification used Apple Clang 17.0.0. `RIVET_CLANG` can select a different compiler executable.

```sh
git clone https://github.com/mrmcgrewx/rivet-lang.git
cd rivet-lang
cargo build --release --locked
./target/release/rivet ui
```

Open **http://127.0.0.1:7331**. No Node.js or frontend build is needed to use the workbench. If the port is occupied, use `./target/release/rivet ui --port 0` and open the printed address. Ctrl-C or SIGTERM stops the workbench and its managed server. Built executables remain on disk.

## Build independent programs

```sh
./target/release/rivet build examples/hello.rivet.json -o ./hello
./hello
# Hello, world!

./target/release/rivet build examples/server.rivet.json -o ./server
./server
# In another terminal:
curl -i http://127.0.0.1:8787/hello
```

The server returns HTTP 200, JSON content type, and `{"message":"hello world"}`. Ctrl-C stops it. Copy either executable to another directory and run it there: it needs ordinary macOS system libraries, without Rivet, Clang, source, sidecars, or a workbench process.

Build preserves `.ll` (LLVM), `.s` (assembly), and `.build.json` (metadata) beside the executable. It refuses to overwrite existing outputs. Choose a new output path for another build. These are host-target artifacts, not universal macOS binaries or signed distribution packages.

## Workbench

Load **Hello world**, edit the program, choose **Build**, then **Run executable**. Build links a native artifact; Run launches that file as an OS process and reports its exit code. The **Native executable** card shows its path, size, profile, target, and download action. **Compiled code** switches between typed registers, LLVM IR, and assembly. Downloaded copies may need `chmod +x rivet-program`.

Load **HTTP server**, Build, then **Start server**. The UI shows its address and process ID. Send `/hello` through the request inspector to make a real TCP request, then Stop. Edit the text/route/status and rebuild to change native behavior. Port `0` in the program selects an available port.

Source edits, accepted builds, and running servers have separate state. Building never changes an existing server; Stop remains available after invalid edits. Artifacts live under `.rivet/builds/` and survive workbench exit. The UI's build history and unsaved editor state do not survive reload. **Download source** saves the program representation; **Download executable** saves the native program.

## Language and generated-code boundary

Rivet programs are typed single-assignment graphs, serialized as topologically ordered JSON instructions. Stable node IDs and explicit operand edges make edits local and mechanically checkable. The compiler validates schema, types, unique IDs, dependency order, entry capabilities, and resource limits. This is a design rationale for AI construction, not evidence that models outperform conventional languages with it. Human inspection is supported; deliberate unreadability is not a language property.

Seven operations produce `Text`, signed 64-bit `Int`, `Bool`, and `Json`: `text`, `int`, `bool`, `concat`, checked `add`, `to_text`, and `object`. LLVM receives explicit types, dependencies, checked-overflow branches, and ordered calls. Generated code directly invokes text/JSON helpers and program-defined route registration. C support supplies allocation, formatting, output, sockets, and bounded HTTP transport. It contains no Rivet parser or instruction interpreter. Both examples use this same path.

- [Language semantics](docs/LANGUAGE.md)
- [Architecture and representation rationale](docs/ARCHITECTURE.md)
- [Native ABI and acceptance contract](docs/NATIVE-PLAN.md)
- [API contract](docs/CONTRACT.md)
- [Validation and reproducibility](docs/NATIVE-VALIDATION.md)
- [Project state and milestone history](docs/STATE.md)

## Efficiency objective

The initial objective is **standalone executable file size**. Default `size` uses Clang `-Oz` and linker dead-code stripping; `--profile baseline` uses the same LLVM and C support at `-O0` without requested dead stripping. On the verified host, three-sample medians showed 5.4% smaller Hello, 0.65% smaller server, and a tie for the computed example, with slower builds. These tiny examples do not establish broad compiler or execution superiority. [Raw measurements](docs/NATIVE-BENCHMARK.json) include timing scope and samples.

```sh
python3 scripts/native_benchmark.py
```

## Checks and command-line tools

```sh
./target/release/rivet check examples/hello.rivet.json
./target/release/rivet run examples/hello.rivet.json
./target/release/rivet interpret examples/hello.rivet.json
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
python3 tests/native_runtime/run.py
python3 tests/native_runtime/run.py --sanitize undefined
```

`check` prints validated registers; `run` builds and launches a persistent native artifact; `interpret` explicitly invokes the reference interpreter. Failures return nonzero. Rust dependencies are locked in Cargo.lock. Browser tests additionally require Node.js 22 or newer:

```sh
npm ci
npx playwright install chromium
npm test
```

## Limits

Straight-line language: no user branches, loops, functions, persistent storage, or request-dependent computation. HTTP responses are computed at startup. Strings/JSON/output have 64 KiB bounds, source has a 64 KiB bound, and programs permit 256 instructions and 32 routes; see the language specification for details. Duplicate JSON object keys use last-value semantics; duplicate instruction IDs and route paths are rejected.

HTTP is a local experimental subset: IPv4 loopback, exact GET routes, sequential connections, bounded headers/body, a two-second read deadline, and no TLS, chunked requests, streaming, or production hosting. The workbench checks local Host/Origin and a custom request header. Other platforms, distribution signing, model-generation efficiency, execution-speed improvements, and memory-use improvements remain unverified. A handwritten machine-code emitter is outside this milestone.
