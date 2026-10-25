# Rivet v1 language

Rivet is a small typed, single-assignment dataflow language. JSON is its source
serialization: each instruction defines one value, operands name earlier
instructions, and an entry selects either console output or a local HTTP service.
Rivet programs are parsed and type checked, lowered to an indexed register IR,
then emitted as LLVM and compiled/linked to native code with Clang. A Rust
interpreter executes the same validated IR as a correctness reference. Programs
are never evaluated as JavaScript or interpolated host source.

## Source structure

A source file is a UTF-8 JSON object with these four required fields:

```json
{
  "version": 1,
  "name": "Greeting",
  "instructions": [
    { "id": "message", "op": "text", "value": "Hello, world!" }
  ],
  "entry": { "kind": "console", "value": "message" }
}
```

`version` must be the integer `1`. `name` contains 1–128 UTF-8 bytes. The
`instructions` array determines execution order. An ID is 1–64 ASCII characters,
starting with a letter or underscore and continuing with letters, digits, or
underscores. IDs must be unique. References can name only earlier valid
instructions; self references, forward references, and missing IDs are errors.

Unknown fields are errors on program, instruction, entry, and route objects.
Instruction fields depend on the selected operation. The keys in an `object`
instruction's `fields` are user-defined JSON keys, so arbitrary string keys are
allowed there. Field order does not affect semantics. JSON source comments and
trailing commas are invalid. Source is parsed by `serde_json`; duplicate JSON
object keys follow that parser's last-value behavior. Use unique JSON keys to
avoid ambiguity. Duplicate instruction IDs and route paths are always rejected.

## Types and instructions

| Operation | Required fields besides `id` and `op` | Result | Semantics |
| --- | --- | --- | --- |
| `text` | `value`: string | `Text` | Exact UTF-8 text; no interpolation. |
| `int` | `value`: integer | `Int` | Signed 64-bit integer, from −9223372036854775808 to 9223372036854775807. |
| `bool` | `value`: boolean | `Bool` | JSON `true` or `false`. |
| `concat` | `values`: array of `Text` IDs | `Text` | Join values in the supplied order without separators. An empty list produces empty text. |
| `add` | `left`, `right`: `Int` IDs | `Int` | Checked signed addition. Overflow is an execution error. |
| `to_text` | `value`: `Text`, `Int`, or `Bool` ID | `Text` | Preserve text; use signed decimal integer formatting; format booleans as `true` or `false`. |
| `object` | `fields`: object mapping JSON keys to IDs | `Json` | Construct a JSON object; preserve text, integer, boolean, and nested object types. An empty object is valid. |

There are no implicit conversions. `concat` does not accept integers until they
are converted with `to_text`. `to_text` does not accept `Json`. An integer source
literal cannot use fractional or floating-point notation, including `1.0`.

Every instruction runs exactly once, in source order, each time a compiled
program executes. This includes instructions unused by the entry. An error in an
unused instruction still fails execution. There are no loops, branches,
recursion, mutable variables, arrays, null values, files, subprocesses, or
external network operations in v1.

## Console entry

```json
{ "kind": "console", "value": "message" }
```

The referenced value must have type `Text`. Execution produces that exact
text followed by one newline. A text value already ending in a newline therefore
produces an additional newline. The runtime emits the complete output only after
successful execution.

## HTTP entry

```json
{
  "kind": "http",
  "port": 8787,
  "routes": [
    { "method": "GET", "path": "/hello", "status": 200, "body": "response" }
  ]
}
```

`port` is an integer from 0 through 65535. Port `0` asks the operating system to
choose an available port. The runtime always binds to IPv4 loopback
(`127.0.0.1`). There can be up to 32 routes, including zero. Each route requires:

- `method`: exactly `GET`.
- `path`: a unique exact ASCII path of at most 2,048 bytes beginning with `/`,
  without whitespace, control characters, a query (`?`), or a fragment (`#`).
- `status`: an integer from 200 through 299, excluding bodyless statuses 204 and
  205.
- `body`: the ID of a `Json` value created by `object`.

The runtime serves the program-defined body with content type
`application/json; charset=utf-8`. It matches paths exactly after removing the
request query string. Missing routes return 404; unsupported methods return 405.
Percent escapes in paths are literal characters rather than instructions to the
language compiler to decode the path.

All route bodies are computed once when the program starts, and then served for
each request. The v1 language cannot inspect request data or maintain request
state. The interpreter returns an HTTP service definition; the runtime owns
binding, requests, and shutdown.

## Compilation, IR, and diagnostics

Compilation validates the complete program before producing an executable
artifact. IDs are replaced by indexed, typed registers. For the Hello world
example, the IR includes:

```text
rivet-ir v1 "Hello world"
%0: Text = text "Hello, "    ; greeting
%1: Text = text "world!"    ; audience
%2: Text = concat [%0, %1]    ; message
entry console %2
```

The actual IR uses private Rust instruction enums, distinct `Text`, `Int`,
`Bool`, and `Json` register wrappers, and typed entry references. This text is a
deterministic inspection view, not executable source or a saved bytecode format.
Object fields are lowered in sorted key order. Compilation does not perform
custom constant folding or dead-code elimination at this stage. Native lowering
then emits LLVM with checked overflow branches and direct C helper calls; Clang
optimizes and generates native machine code. Build saves LLVM and assembly for
inspection. See `NATIVE-PLAN.md` for the native ABI and runtime boundary.

Build diagnostics contain `path` and `message`, for example:

```json
{
  "path": "$.instructions[2].values[1]",
  "message": "expected Text, found Int"
}
```

Schema and type diagnostics can include multiple errors from one build. A later
reference to an invalid earlier instruction can produce an additional reference
diagnostic. JSON syntax errors use path `$` and include a line and column.
Invalid programs have no executable artifact.

Execution errors are separate from compilation diagnostics. Addition overflow
identifies the instruction ID and register. Native computed-value size errors
identify the failed operation/limit; the reference interpreter also includes
instruction context. A
console-output size error identifies the entry output limit. Execution errors
return no partial console output or service definition.

## Resource limits

| Resource | Limit | Enforcement |
| --- | --- | --- |
| Source | 65,536 UTF-8 bytes, including whitespace | Before JSON parsing |
| Name | 1–128 UTF-8 bytes | Compilation |
| Instructions | 256 | Compilation |
| Instruction ID | 1–64 ASCII characters | Compilation |
| One literal `Text` | 16,384 UTF-8 bytes | Compilation |
| One computed `Text` | 65,536 UTF-8 bytes | Execution |
| One computed `Json` | 65,536 serialized JSON bytes | Execution |
| Console output | 65,536 UTF-8 bytes, including the appended newline | Execution |
| HTTP routes | 32 | Compilation |
| HTTP route path | 2,048 ASCII bytes, including the leading `/` | Compilation |

JSON size includes braces, commas, keys, colons, string quotes, and escaping.
For example, a control character can occupy one byte in `Text` but six bytes as a
JSON `\u0000` escape. Nested objects count in full. Concatenation checks the
result size before allocating it. Object construction checks each field before
retaining its cloned value, preventing repeated nested references from growing
an unchecked output object. Source parsing also retains `serde_json`'s default
recursion-depth limit.

These limits define bounded prototype behavior; they are not performance claims
or a complete security sandbox for executing hostile programs.

## Examples and checks

- [`hello.rivet.json`](../examples/hello.rivet.json) concatenates two text values
  and prints `Hello, world!`.
- [`server.rivet.json`](../examples/server.rivet.json) defines `GET /hello` on
  port 8787 with body `{"message":"hello world"}`.
- [`computed.rivet.json`](../examples/computed.rivet.json) adds 40 and 2,
  converts the result to text, concatenates a message, and builds nested JSON
  with integer and boolean fields. `GET /computed` uses an assigned port.

From the repository root:

```sh
cargo test --locked --test language
cargo run --locked -- check examples/hello.rivet.json
cargo run --locked -- run examples/hello.rivet.json
cargo run --locked -- run examples/server.rivet.json
```

The focused test suite verifies executed examples and edits, deterministic typed
lowering, value composition, literal and operand types, error paths, boundary
limits, overflow, HTTP entry validation, and repeatable execution. Network and
browser lifecycle validation belong to the runtime and workbench suites.
