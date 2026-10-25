# Architecture and decisions

Current architecture, 2026-10-25. Native compilation is core to completion. The original interpreter-only implementation was useful groundwork but did not satisfy the intended experiment.

## Representation

The canonical program is a typed acyclic dataflow graph, serialized as a topologically ordered JSON instruction list. Stable IDs and explicit dependencies let an agent replace values, add nodes, or reconnect operands with localized edits. JSON uses existing parsers and supports inspection without introducing a new punctuation grammar. It is only serialization: language semantics define four types, seven operations, checked arithmetic, ordered failures, limits, and console/HTTP effects.

A binary encoding would reduce source bytes but would obscure inspection and require another decoding tool without demonstrated benefit at this scale. A graph preserves dependency structure; a topological list gives deterministic validation and failure order. AI construction convenience is a hypothesis, not a measured superiority claim. The language is machine-oriented rather than literally unreadable by humans.

The validator checks exact schema, unique IDs, backward-only references, operand types, scalar ranges, bounded literals/instruction counts, and legal entries/routes. Values have explicit types and widths. Pure dependencies and distinct output/network effects survive lowering. Every instruction is evaluated in source order, including unused nodes that can fail, so optimization must preserve observable failure behavior.

## Native compilation

`src/language.rs` lowers source to a private typed indexed IR and provides a reference interpreter. `src/language/native_codegen.rs` emits LLVM directly from this IR, without running the interpreter. Scalar values become LLVM constants/values; checked addition uses `llvm.sadd.with.overflow.i64` with explicit error branches. Text, conversion, and object operations become direct calls to the C ABI in `native/runtime.h`. HTTP entries emit their own route registration, status, body, and port before calling the transport loop.

`src/native.rs` embeds the C support sources, writes LLVM and temporary support files, invokes Clang for object/assembly generation and linking, and publishes an executable plus LLVM, assembly, and metadata sidecars. Backend processes have time/output bounds. Existing destinations are rejected. The established LLVM/Clang backend handles optimization, register allocation, and machine-code generation; writing our own emitter is not part of this milestone.

The verified target is arm64 Mach-O on macOS. Executables link native support and use only system libraries. They do not contain the Rust interpreter, a source parser, an instruction dispatcher, or a dependency on source/sidecar files. Compiler-side Rust dependencies do not become executable runtime dependencies. C support is embedded in the compiler, so builds also work outside the source checkout.

## Runtime boundary and memory

Generated code owns program operations and entry configuration. C helpers own immutable byte values, concatenation, number/boolean formatting, exact JSON encoding, stdout, allocation cleanup, and HTTP sockets/protocol handling. Strings use byte lengths and preserve embedded NUL and UTF-8. Integers remain exact across the signed 64-bit range. Object keys use deterministic ordering. Helpers enforce computed-value/output bounds before unbounded growth.

Values have process lifetime during bounded straight-line computation and are freed on normal exit. Server responses are computed once at startup; no new language values are evaluated per request. C transport handles loopback binding, exact paths/query stripping, 404/405, framing/size/deadline checks, SIGINT/SIGTERM, and clean socket release. `RIVET_CONTROL=1` enables parent-only request-count messages on stderr. Without it, standalone serving needs no parent protocol.

The older Rust HTTP adapter and interpreter server remain for the workbench API and reference tests. An earlier `tiny_http` adapter was replaced after review demonstrated shutdown blocking on incomplete request bodies; owned sockets now have bounded parsing and cancellation. Native transport independently implements the same documented small HTTP subset. Neither adapter is a production HTTP server.

## Workbench and lifecycle

The embedded HTML/CSS/JS UI edits source, inspects diagnostics/typed IR/LLVM/assembly, and exposes executable metadata/download. Build succeeds only after native linking. Persistent files live under `.rivet/builds/<session>/<id>/`; in-memory build state is separate from source edits and from an already running process.

`src/native_process.rs` launches artifact paths as OS children, captures bounded output/exit status, and manages a server by its readiness line and owned PID. Stop sends SIGTERM, waits, then escalates on a bounded timeout and reaps the child. Status detects unexpected exit. Workbench SIGINT/SIGTERM stops its child; artifacts remain usable independently. The UI request inspector makes actual TCP requests. No example output is hardcoded into the workbench.

## Dependencies and efficiency

Rust 1.90.0, Cargo.lock, and package-lock.json pin build/test dependencies. serde/serde_json parse source and reference values; httparse parses workbench/reference HTTP; ctrlc with termination support handles SIGINT/SIGTERM/SIGHUP; libc supports native-process signals. Native production support is C11 plus system libraries. The frontend has no runtime framework; Playwright is test-only.

The initial performance objective is executable file bytes. Compare identical emitted LLVM and C support under baseline `-O0` and size `-Oz` with linker dead stripping. Record actual size plus compilation wall time, preserve semantics under both profiles, and report ties/regressions. This evaluates a conventional backend policy, not a novel optimizer or universal performance claim. See `NATIVE-BENCHMARK.json` and `NATIVE-VALIDATION.md`. The old Rust interpreter benchmark remains historical reference work.
