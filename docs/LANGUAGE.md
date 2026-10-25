# Rivet language semantics — RVB1 binary source

The canonical program is an actual binary `.rvb` file. The [RVB1 format](BINARY-FORMAT.md) defines exact header, opcode, type, payload and entry bytes. There is no required text syntax, embedded JSON source, or name table. A readable disassembly is an inspection view generated from the file. CLI `inspect`, `check`, `build`, `run`, `interpret` and the GUI all consume binary bytes.

## Values and operations

Each instruction defines one immutable value. Its zero-based position is its numeric register ID; operands may reference only earlier definitions. Values have types `Text`, `Int`, `Bool`, and `Json`. The record declares a type tag which must match its operation. There are no implicit conversions.

| Opcode | Inspection name | Result | Meaning |
| --- | --- | --- | --- |
| `01` | `text` | Text | Exact UTF-8 payload, including embedded NUL. |
| `02` | `int` | Int | Signed 64-bit little-endian two's-complement literal. |
| `03` | `bool` | Bool | One byte, exactly 0 or 1. |
| `10` | `concat` | Text | Concatenate referenced Text values in operand order. Empty gives empty text. |
| `11` | `add` | Int | Checked signed addition of two Int values; overflow fails execution. |
| `12` | `to_text` | Text | Text identity, signed decimal integer formatting, or `true`/`false` for Bool. Json is rejected. |
| `20` | `object` | Json | Construct an object from sorted unique UTF-8 keys and typed value references; preserve scalar/nested object types. |

Hexadecimal opcode values are notation in this specification; the file stores numeric bytes. Operation names, `%0` register labels, and readable comments are produced by the inspector, not stored source tokens. Object keys must be strictly increasing by UTF-8 byte order and may include Unicode/control characters. Duplicate keys and noncanonical field order are rejected.

All instructions evaluate once in source order, including unused instructions. An error in an unused instruction still fails execution. Optimizations must preserve observable failure order. There are no loops, user branches, recursion, mutable variables, arrays, null, filesystem operations, subprocess operations, or external network operations in the language.

## Entries

A console entry (header tag 1) names one Text register. Output is its exact bytes plus one newline. A value ending in newline therefore produces two. Computation must succeed before console output starts.

An HTTP entry (tag 2) contains a port and zero to 32 routes. Each route stores an ASCII path, status and Json body reference. GET is implicit. Port 0 requests an available port; all listeners bind IPv4 loopback. Paths start with `/`, have at most 2048 bytes, and exclude whitespace, controls, query/fragment markers and non-ASCII bytes. Paths are unique. Status may be 200–299 except 204/205, whose bodies would be invalid.

Route values are computed once at server startup. The runtime serves exact JSON with content type `application/json; charset=utf-8`. Paths match exactly after query stripping. Missing routes yield 404; unsupported methods yield 405. Percent escapes remain literal. Requests cannot affect program values in this version.

## Validation, execution, and inspection

The decoder validates the full file before producing typed IR. It checks the magic/version/flags, declared lengths, reserved fields, opcode/result tags, literal encodings, references/types, field ordering, entries and bounds. It rejects truncation and all trailing bytes. Diagnostics identify an absolute byte offset and the error; no JSON parser or host-language evaluation occurs.

`rivet inspect examples/hello.rvb` decodes the two literals, concatenation and console entry into readable typed registers. The workbench uses the same inspection API and absolute record/payload offsets to edit bytes. Raw hexadecimal editing can change any instruction; saving always writes binary bytes, not a hex-text file. Invalid edits remain inspectable as raw bytes but cannot build.

`compile_binary` lowers directly into the same private typed IR used by native generation and the reference interpreter. Native lowering emits LLVM checked arithmetic and calls to C helpers. Clang performs optimization and code generation. Artifacts preserve `.ll`, `.s` and `.build.json` for inspection. Runtime overflow and computed-value errors remain distinct from structural/type errors. Native helper diagnostics name the failed operation/limit; checked arithmetic also identifies the register.

`encode_binary` deterministically writes typed IR back into the canonical binary encoding. It is used for roundtrip/testing and is not a prerequisite for writing source. The optional fixture writer authors opcode bytes directly. The old JSON compiler remains a library compatibility/test path; public source commands do not use it, and its files live under `tests/fixtures/legacy/`.

## Resource limits

| Resource | Limit | Enforcement |
| --- | --- | --- |
| Complete binary source | 65,536 bytes | Before decoding |
| Instruction count | 256 | Decoding |
| One literal Text | 16,384 UTF-8 bytes | Decoding |
| One computed Text | 65,536 bytes | Execution |
| One computed Json | 65,536 serialized JSON bytes | Execution |
| Console output including newline | 65,536 bytes | Execution |
| HTTP routes | 32 | Decoding |
| HTTP route path | 2048 ASCII bytes | Decoding |

Record payload lengths and register/count fields are unsigned 16-bit. Header body length is unsigned 32-bit but must match the bounded file exactly. Decoder loops/allocations are constrained by remaining input and instruction/route limits. Computed JSON size includes escaping, keys and recursively nested objects. These are per-value bounds, not an aggregate hostile-program memory quota.

HTTP transport permits 8 KiB headers, 512 KiB bodies, one request per connection, and a two-second read deadline. Framing ambiguity/Transfer-Encoding are rejected. SIGINT/SIGTERM stops native services cleanly; SIGKILL/crashes have no graceful-cleanup guarantee.

## Examples

- [`hello.rvb`](../examples/hello.rvb): 49 binary bytes, prints `Hello, world!`.
- [`server.rvb`](../examples/server.rvb): 64 binary bytes, GET `/hello` on port 8787 returning `{"message":"hello world"}`.
- [`computed.rvb`](../examples/computed.rvb): 159 binary bytes, adds 40+2, converts/concatenates text, constructs nested typed JSON, serves `/computed` on an assigned port.
