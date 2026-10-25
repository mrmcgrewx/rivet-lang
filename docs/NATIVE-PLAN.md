# Native artifact milestone — revised completion contract

Updated 2026-10-25. This supersedes interpreter-only completion claims in the historical v0 validation. Native compilation is required, not optional follow-up work. Preserve existing source semantics, validator, reference interpreter, and workbench foundations.

## Design and rationale

Rivet's canonical program is a typed acyclic dataflow graph with named nodes, serialized as a topologically ordered instruction list in JSON. Stable IDs let an agent replace a node or reconnect an operand without inventing syntax, parsing indentation, or rewriting unrelated nodes. The serialization stays inspectable and round-trippable; making it opaque provides no engineering benefit. These are design conveniences, not a measured claim of better model reliability.

The compiler validates the schema, value types, unique definitions, prior-definition dependencies, entry capabilities, and resource bounds. The graph preserves scalar widths, checked signed arithmetic, ordered potentially-failing operations, string/JSON value boundaries, and pure-data dependencies. Only the entry introduces output/network effects. Unused instructions can fail; optimizations must preserve those failures. A typed register rendering, emitted LLVM IR, and generated assembly are inspection views of the same program.

The first native compiler lowers the validated register form directly to LLVM IR. Integer addition uses the checked signed-overflow intrinsic and explicit failure branches. Text/JSON operations become direct calls to narrowly defined native support routines; there is no source parser, bytecode dispatcher, or interpreter in the executable. LLVM through the locally installed Clang performs optimization, machine instruction selection, register allocation, object generation, and linking. A handwritten machine-code emitter is explicitly out of scope for this milestone.

The artifact is a Mach-O arm64 executable on the primary macOS/Apple Silicon target. It embeds native support code and depends only on ordinary system libraries, not Rivet, Rust, JSON source files, the repository, or a running workbench. Build also preserves emitted `.ll`, assembly `.s`, and build metadata next to the executable for inspection. Runtime C source is embedded in the compiler so building does not depend on the repository's source path.

The memory model is immutable values with process-lifetime allocation for the bounded straight-line computation. Scalar values are native i64/i1 values. UTF-8 strings carry explicit byte lengths, including embedded NUL. JSON remains exact, including all signed 64-bit integers and deterministic key ordering. Values/console output obey the existing 64 KiB bounds. HTTP responses are computed at startup in this revision; request-dependent computation remains optional subsequent language work.

## Native support ABI v1 (fixed before delegation)

All functions use the platform C ABI and opaque pointers to immutable byte values. Every routine exits nonzero on invalid size/allocation/I/O failures, writing a useful diagnostic to stderr. No routine parses a Rivet program or executes an operation stream. Function names/signatures in LLVM and C must match exactly:

```c
void *rv_text(const unsigned char *bytes, uint64_t length);
void *rv_concat(void *const *values, uint64_t count);
void *rv_i64_text(int64_t value);
void *rv_bool_text(uint8_t value);
void *rv_json_text(void *text);
void *rv_json_i64(int64_t value);
void *rv_json_bool(uint8_t value);
void *rv_object(const unsigned char *const *keys, const uint64_t *key_lengths,
                void *const *json_values, uint64_t count);
void rv_print(void *text); // exact bytes plus newline; final output <=65536 bytes
_Noreturn void rv_fail(const unsigned char *message, uint64_t length);
void rv_add_route(const unsigned char *path, uint64_t length, uint32_t status, void *json);
int32_t rv_serve(uint32_t port);
void rv_cleanup(void); // free all support allocations on normal exit
```

C support: `native/runtime.c` and `native/runtime.h`. Integer overflow checking is in generated LLVM, not a C add helper. Every source instruction is emitted in order; console entry calls rv_print, HTTP entry registers each program-defined route then calls rv_serve. Native main calls rv_cleanup before returning. Scalar literals may lower to LLVM constants. Emit escaped UTF-8 byte arrays, never interpolate source strings as code. LLVM opaque pointer form is supported by the available Apple Clang 17.

HTTP support binds IPv4 loopback, permits port 0, handles exact GET paths ignoring query strings, returns program status/body with JSON content type, and returns 404/405 for missing routes/methods. It bounds request headers (8 KiB), body (512 KiB), path (2048 bytes), and total read time (2 seconds), handles partial clients, closes each connection, and releases the listener on SIGINT or SIGTERM. Ready line on stdout is exactly `Rivet program server: http://127.0.0.1:<port>` plus newline and flush. When environment variable RIVET_CONTROL equals `1`, emit `RIVET_REQUEST` plus newline to stderr per handled request; no logging by default. stdout from console programs is only program output.

## Rust interfaces and ownership

- Compiler agent owns additions in `src/language.rs`, new `src/language/native_codegen.rs` (child module may access parent's private typed IR), `tests/native_codegen.rs`, and `docs/NATIVE-CODEGEN-HANDOFF.md`. Expose `CompiledProgram::emit_llvm() -> String` and `CompiledProgram::entry_info() -> EntryInfo`; `EntryInfo` is public Clone + Debug + Serialize, internally tagged kind lowercase: Console or Http { port: u16, routes: Vec<String> }. Do not alter reference semantics. Emit LLVM for every current operation, with tests against shape/escaping and native differential checks once builder is ready. No native builder, runtime C, UI, or shared-doc edits.
- Runtime agent owns `native/runtime.c`, `native/runtime.h`, `tests/native_runtime/` harnesses if needed, and `docs/NATIVE-RUNTIME-HANDOFF.md`. Implement ABI and HTTP lifecycle with strict C warnings and native/sanitizer checks. No Rust or shared-doc changes.
- UI agent owns `web/`, `tests/ui/`, and `docs/NATIVE-UI-HANDOFF.md`. Show executable artifact path/size/profile, download action, LLVM/assembly inspection, exit status and native process state; retain existing controls and source/build/running separation. Update/add browser tests for artifact visibility/download. No Rust/C/package edits unless coordinated.
- Native review agent reads all new compiler/runtime/integration/UI code and tests, writes only `docs/NATIVE-REVIEW.md`, and reports actionable correctness/acceptance gaps. No implementation or Git mutations.
- Coordinator owns `src/native.rs`, `src/native_process.rs`, CLI/workbench/HTTP adapter integration, Cargo files, native differential/artifact tests, benchmark script, shared docs, commits, and final validation. Coordinator commits all contributions; no concurrent Git index operations.

## Workbench/API changes

Existing response fields remain; Build now succeeds only after native compilation/link succeeds. Build info adds:

```json
{"artifact":{"path":"/absolute/.../program","size_bytes":123,"profile":"size","target":"...","download_url":"/api/artifacts/1"},"llvm_ir":"...","assembly":"...","build_ms":123}
```

Artifact storage defaults to `.rivet/builds/<unique-session>/<build-id>/program` under the working directory, ignored by Git. Files survive workbench exit. CLI output path is explicit. Download serves executable bytes with application/octet-stream and attachment name `rivet-program`; downloaded copies may need `chmod +x`.

Run starts that artifact as an OS child process, never the reference interpreter. Console response adds `exit_code`; failures report stderr. Server info adds `pid` and is derived from the executable's readiness line. Stop sends SIGTERM, waits/reaps, and escalates only after a bounded timeout. Status detects exited children. Workbench shutdown stops its managed child but does not delete artifacts. A user can launch the artifact independently afterward. Request inspector continues to use real TCP against the native service.

CLI: `rivet build FILE -o PATH [--profile size|baseline]`, `rivet run FILE` builds then launches native artifact, `rivet interpret FILE` explicitly uses reference interpreter, existing check prints validated IR. The default profile is size. Print artifact/LLVM/assembly/metadata paths after build.

## Efficiency objective and baseline

Primary objective: small independently deployable executable artifacts, preserving exact v1 semantics. Default size profile uses Clang -Oz plus linker dead-code stripping. Baseline uses the same generated LLVM and runtime with -O0 and no requested dead stripping. This isolates a straightforward optimization policy; it does not claim a novel optimizer or universal superiority. Measure file byte size and native build wall time on the same machine for hello, server, and computed examples. Build-time tradeoff and alignment floors may limit gains; report ties/regressions honestly. Differential checks must pass for both profiles before size results count.

## Acceptance

1. Build Hello from UI and CLI; file inspection confirms arm64 Mach-O; `./hello` prints exact output. Changed program changes native output; invalid programs fail before artifact acceptance. Copy executable alone to a new directory, close workbench, and run with no source/Rivet runtime available.
2. Build server through the same path; run copied executable independently, repeat real JSON HTTP requests, verify program-defined status/routes/payload and clean shutdown/rebind.
3. UI exposes artifact paths/download and native code views; Run launches executable and reports exit/failure/lifecycle. Captured actual UI evidence plus automated browser coverage.
4. Compare reference/native outputs across meaningful generated programs, all operations, Unicode/NUL/escaping, i64 boundaries/overflow, size errors, nested objects, both profiles, and HTTP responses. No source/interpreter fallback.
5. Clean checkout setup includes Rust and installed Clang/Xcode command-line tools, with exact verified commands and backend version. Benchmark the defined objective against baseline. Record current scope honestly and push meaningful checkpoints.
