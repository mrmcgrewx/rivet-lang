# Rivet v1 implementation contract

Committed before delegation. Changes require coordination and an update here.

## Language

Programs are UTF-8 JSON objects with exactly `version` (1), `name` (nonempty string), `instructions` (array), and `entry`. JSON is the serialization for a custom typed, single-assignment dataflow language, not executable JavaScript or host-language source.

Every instruction defines a unique `id` (ASCII letter/underscore followed by ASCII letters, digits, underscore; max 64 characters). Operands reference earlier IDs only. Supported instructions:

| op | other fields | result |
| --- | --- | --- |
| text | `value`: string | Text |
| int | `value`: signed 64-bit integer | Int |
| bool | `value`: boolean | Bool |
| concat | `values`: array of Text register IDs | Text |
| add | `left`, `right`: Int IDs | Int (checked overflow) |
| to_text | `value`: Text, Int, or Bool ID | Text |
| object | `fields`: object mapping JSON keys to register IDs | Json (nested Json allowed) |

Entry is either `{"kind":"console","value":"id"}` (requires Text, writes exactly that text plus newline), or `{"kind":"http","port":8787,"routes":[{"method":"GET","path":"/hello","status":200,"body":"id"}]}` (body requires Json). Port 0 requests an OS-assigned port. Ports are unsigned 16-bit. HTTP routes require unique exact paths, GET only, status 200..299 except bodyless 204/205; paths start with `/`, exclude query/fragment, whitespace, controls, and non-ASCII. HTTP JSON content type is supplied by the runtime; the program defines the body, status, route, and port. Missing routes return 404. Query strings are ignored for route matching. Other methods return 405.

Reject unknown fields, unknown operations, duplicate instruction IDs, missing/forward operands, bad types, invalid version, and invalid entries. Diagnostics have JSON fields `path` and `message`; syntax errors include line/column in the message. Limits: source 64 KiB, 256 instructions, name 128 bytes, literal text 16 KiB, computed value/output 64 KiB, 32 routes. All instructions execute in order exactly once when a program runs or starts a server. No loops, recursion, files, subprocesses, or external network calls. No native-code optimization claims.

## Rust language module API (language owner)

`src/language.rs` exports:

```rust
#[derive(Clone, Debug, serde::Serialize)]
pub struct Diagnostic { pub path: String, pub message: String }
pub fn compile(source: &str) -> Result<CompiledProgram, Vec<Diagnostic>>;
// CompiledProgram is Clone + Debug; its internal typed IR is private.
impl CompiledProgram {
    pub fn name(&self) -> &str;
    pub fn instruction_count(&self) -> usize;
    pub fn disassemble(&self) -> String;
    pub fn execute(&self) -> Result<Execution, String>;
}
#[derive(Clone, Debug)]
pub enum Execution {
    Console { stdout: String },
    Http { port: u16, routes: Vec<Route> },
}
#[derive(Clone, Debug)]
pub struct Route { pub method: String, pub path: String, pub status: u16, pub body: serde_json::Value }
```

Lower symbolic references to indexed, typed register instructions. Do not execute raw source or string-evaluate host code. Build-time errors and runtime overflow/size errors are distinct. Examples are `examples/hello.rivet.json` and `examples/server.rivet.json` with the traditional outputs from the brief. A computed example exercises addition, conversion, concatenation, and object composition. Language tests live in the same module and/or `tests/language.rs`.

## Workbench API (coordinator owns backend; UI owner consumes)

Default workbench address: `http://127.0.0.1:7331`; `rivet ui --port 0` is supported for tests. Default program server port: 8787, editable in the program. All HTTP listeners use IPv4 loopback. Each workbench owns one build and at most one running program server. Building never implicitly stops an existing server. UI must distinguish edited source, built artifact, and running server. Stop remains available regardless of editor validity.

Requests and responses use JSON. POST requests require `Content-Type: application/json` and `X-Rivet-Request: 1`. Reject foreign Origin/Host headers to protect local mutation. Browser receives no cross-origin API permission. Program source size is limited. API errors: `{"ok":false,"error":"...","diagnostics":[]}` (diagnostics populated for build errors).

- `GET /api/examples` → `{"examples":[{"id":"hello","name":"Hello world","source":"..."},{"id":"server","name":"HTTP server","source":"..."},{"id":"computed","name":"Computed values","source":"..."}]}`.
- `GET /api/status` → `{"ok":true,"build":null|{"id":1,"name":"...","instructions":2,"ir":"..."},"server":null|{"address":"http://127.0.0.1:8787","requests":0,"routes":["/hello"],"build_id":1},"last_stdout":"..."}`.
- `POST /api/build` body `{"source":"..."}` → `{"ok":true,"build":{"id":1,"name":"...","instructions":2,"ir":"..."},"diagnostics":[]}`. Invalid build clears current build, returns HTTP 400 diagnostics; already-running server is preserved. Build IDs monotonically increase per process.
- `POST /api/run` body `{"build_id":1}` → console `{"ok":true,"kind":"console","stdout":"Hello, world!\n","elapsed_us":123}` or server `{"ok":true,"kind":"http","server":{"address":"...","requests":0,"routes":["/hello"],"build_id":1}}`. Reject stale/missing build IDs. Reject starting any program while a server runs, requiring explicit Stop. Runtime errors use HTTP 400, lifecycle conflicts HTTP 409.
- `POST /api/stop` body `{}` → `{"ok":true,"stopped":true|false}`. Return only after the listener is released and worker joined.
- `POST /api/request` body `{"path":"/hello"}` → `{"ok":true,"status":200,"content_type":"application/json; charset=utf-8","body":"{\"message\":\"hello world\"}","elapsed_us":123}`. Sends a real TCP HTTP request to the running program server; no arbitrary URL or remote host accepted. Require a path beginning `/`, no whitespace/control/fragment; permit query strings. 404 is an inspectable response, not an API failure.

Serve `/`, `/app.js`, `/style.css` from embedded assets. CLI: `rivet check FILE` prints IR; `rivet run FILE` executes console or starts HTTP service until Ctrl-C; `rivet ui [--port PORT]` starts the workbench until Ctrl-C. Exit nonzero on failures. Print selected addresses for port 0. Ctrl-C stops all listeners and joins workers.

## Ownership and integration

- Language agent: `src/language.rs`, `examples/`, `tests/language.rs`, `docs/LANGUAGE.md`, `docs/LANGUAGE-HANDOFF.md`. No Cargo, backend, or UI edits.
- UI agent: `web/`, `tests/ui/`, `package.json`, `package-lock.json`, `playwright.config.js`, `docs/UI-HANDOFF.md`. Implement editor, examples, new/open/download, build/run/stop, diagnostics, output, IR, live status, request inspector, and browser automation tests. No Rust edits.
- Coordinator: Cargo/toolchain files, `src/lib.rs`, `src/main.rs`, `src/runtime.rs`, `src/workbench.rs`, backend/integration tests, CI, README, shared docs, integration, actual UI verification, final review and clean-checkout reproduction.

All agents read AGENTS.md and this committed contract. Shared worktree is safe only within ownership. Agents persist handoff and meaningful test results; coordinator performs guarded commits and pushes for all contributions to avoid concurrent Git index operations.

## Verification plan

Unit tests validate types, errors, deterministic lowering, value composition, overflow/size limits, and both examples. Runtime tests make repeated real HTTP requests, test status/content type/payload, 404/405, port conflicts, shutdown/rebind. API tests exercise build/run/change/error/lifecycle and local-origin protections. Browser tests perform both MVPs, source changes, build errors, repeated requests, and stop/restart. Coordinator also verifies in the actual browser and records evidence. Reproduce from a clean checkout with locked dependencies before marking complete.
