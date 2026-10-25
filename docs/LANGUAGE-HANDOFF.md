# Language implementation handoff

Updated 2026-10-25. Scope: the committed Rivet v1 contract in
[`CONTRACT.md`](CONTRACT.md), read alongside `AGENTS.md`, `BRIEF.md`,
`ARCHITECTURE.md`, and `STATE.md` before implementation.

## Delivered files

- `src/language.rs`: source-size enforcement, JSON parsing, schema and type
  validation, deterministic typed register lowering, IR inspection, and bounded
  interpretation. Exports exactly the public API in the contract.
- `examples/hello.rivet.json`: concatenated `Hello, world!` console output.
- `examples/server.rivet.json`: program-defined `GET /hello`, port 8787, status
  200, JSON body `{"message":"hello world"}`.
- `examples/computed.rivet.json`: checked addition, scalar conversion,
  concatenation, booleans, integers, and nested object composition; `GET
  /computed` on an assigned port.
- `tests/language.rs`: 24 focused integration tests against the public language
  API, including executed examples and output edits.
- `docs/LANGUAGE.md`: complete syntax, semantic choices, IR explanation,
  diagnostic behavior, limits, examples, and check commands.

No Cargo, CLI, runtime, workbench, frontend, shared state, or Git index files were
edited by the language owner. Coordinator owns integration, guarded commits,
pushes, and milestone state.

## Implementation decisions

The compiler validates into private instruction enums with distinct typed
register wrappers. The symbol table resolves an instruction ID to an indexed
register and its result type. The interpreter executes that IR without retaining
or evaluating source expressions. `CompiledProgram` remains `Clone + Debug` with
private internals. Invalid source never yields an artifact.

Object keys lower in sorted order, independent of source key order. The
disassembly uses `%N: Type` registers, escaped literals, source IDs as comments,
and typed console or HTTP entries. It is an inspection format, not a stable
bytecode serialization contract.

Each instruction executes once in order, including unused instructions. Values
are newly allocated for every `execute()` call; returned HTTP bodies can be
mutated without changing the compiled artifact or subsequent runs. Route bodies
are produced at execution/start time, not recomputed per request. Networking and
output emission remain runtime responsibilities.

Limits use UTF-8 byte counts for source, names, literal text, and computed text.
Computed JSON uses complete serialized JSON byte counts, including escaped keys
and values. Console output counts its mandatory newline within the 65,536-byte
limit. These interpretations were reported to and accepted by the coordinator.
Concatenation checks size before allocating the combined string. Object
construction checks each additional field before retaining its cloned subtree.
The maximum checked JSON size is inclusive; tests cover 65,536 bytes succeeding
and 65,537 bytes failing.

`serde_json` supplies syntax diagnostics and its default parsing recursion
limit. Its default duplicate JSON object-key behavior keeps the last value;
this is documented. Duplicate instruction IDs and exact route paths are
independently rejected as required. Empty concatenations, empty JSON objects,
and HTTP entries with zero routes are permitted because the contract does not
require nonempty collections.

Integration review identified that the transport and request inspector accept
paths of at most 2,048 bytes. The compiler now enforces the same route-path
limit so every accepted route can be addressed by that transport. An added test
checks that a 2,048-byte path compiles and executes unchanged, while a 2,049-byte
path fails at `$.entry.routes[0].path`. The coordinator owns the corresponding
contract update.

## Validation actually performed

The first test attempt was blocked while `web/app.js`, embedded by the
workbench, had not yet been written. After the UI owner supplied that asset:

| Command | Observed result |
| --- | --- |
| `cargo test --locked --test language` | Initial suite passed all 23 tests; 0 failed, 0 ignored. Route-path limit follow-up validation is recorded below. |
| `cargo run --locked -- check examples/hello.rivet.json` | Exit 0; printed the expected typed three-register IR and console entry. |
| `cargo run --locked -- run examples/hello.rivet.json` | Exit 0; printed `Hello, world!` followed by a newline. |
| `rustfmt --edition 2024 --check src/language.rs tests/language.rs` | Passed after the final test addition. |
| `cargo clippy --locked --lib -- -D warnings` | No language diagnostic, but overall command failed on a `collapsible_if` warning in coordinator-owned `src/workbench.rs` at then-line 269. Reported to coordinator for correction and a full rerun. Do not count this invocation as passed. |

After the route-path limit follow-up, `cargo test --locked --test language`
passed all 24 tests with the updated `httparse` transport dependency present.
`rustfmt --edition 2024 --check src/language.rs tests/language.rs` also passed.

Test coverage includes source and literal byte limits; instruction count;
identifier validity and length; missing/forward/self references; primitive and
entry type errors; unknown fields and operations; missing required fields;
syntax line/column diagnostics; signed integer boundaries and both overflow
directions; unused-instruction execution; computed Text and Json sizes; JSON
escaping and nested growth; route methods, status, port and path validation;
duplicate and excessive routes; exact IR; nested typed object contents;
scalar-to-text conversion; output changes after source edits; and independent
repeat execution.

## Integration remaining

Coordinator must run the complete build, lint, runtime and workbench tests,
actual browser UI verification, and clean-checkout reproduction. This handoff
does not claim either MVP complete: language tests alone do not verify socket
binding, repeated HTTP requests, deterministic shutdown, or UI behavior.

No public API deviations or unresolved language test failures are known.
Prototype limitations are described in `LANGUAGE.md`: no request-dependent
computation, external capabilities, optimizer, native backend, or executable
artifact persistence. The parser's duplicate-key behavior is explicit rather
than silently presented as strict duplicate rejection.
