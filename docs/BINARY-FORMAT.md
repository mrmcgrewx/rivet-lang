# RVB1 binary source specification

RVB1 is Rivet's canonical, authored source format. A `.rvb` file contains numeric instruction records and register references. It is not a compiled executable, a JSON container, or an encoded textual program. The compiler reads these bytes directly into typed intermediate representation, then emits native code through LLVM/Clang. The same validated IR can be interpreted as a correctness reference.

The source contains no textual opcode names, variable names, source filename, or debug symbol table. Human-readable code is a decoded inspection view supplied by the CLI and workbench. Text literals, JSON keys, and HTTP paths are stored as their ordinary byte values; they can be recognizable in a binary file. This is a binary language, not encryption or a claim that humans cannot reverse engineer it.

## Conventions

- File extension: `.rvb`.
- Every multibyte integer uses **little-endian** byte order.
- Byte offsets are zero-based, measured from the beginning of the file.
- Numeric opcodes and byte examples below are hexadecimal; limits are decimal.
- Registers are unsigned 16-bit indices, assigned by instruction position starting at zero.
- References inside an instruction must name an earlier instruction. Entry references may name any defined register.
- No implicit type conversions, padding, alignment bytes, optional sections, or ignored extensions exist in version 1.
- The entire file may contain at most 65,536 bytes, including the header.
- The decoder rejects unknown, malformed, noncanonical, or trailing data. It never guesses a format or falls back to textual parsing.

## Header

Exactly 16 bytes precede the instruction records:

| Offset | Width | Encoding | Meaning |
| --- | ---: | --- | --- |
| `0x00` | 4 | `52 56 42 00` | Magic: `RVB` followed by NUL. |
| `0x04` | 2 | u16, value `1` | Format version. |
| `0x06` | 2 | u16, value `0` | Reserved flags. Other values are invalid. |
| `0x08` | 2 | u16 | Instruction count, from 0 through 256. |
| `0x0a` | 1 | u8 | Entry kind: `1` console or `2` HTTP. |
| `0x0b` | 1 | u8, value `0` | Reserved. |
| `0x0c` | 4 | u32 | Body length: exactly the total file length minus 16. |

The body contains exactly the declared instruction count, followed by the selected entry payload. No bytes may follow the entry.

## Instruction records

Each record consists of:

```text
opcode:u8 | result_type:u8 | payload_length:u16 | payload:payload_length bytes
```

The result register is the record's zero-based ordinal; no register ID is stored in the record. A record's declared result type must match its opcode. Type tags are `1 = Text`, `2 = Int`, `3 = Bool`, and `4 = Json`.

| Opcode | Result | Payload |
| --- | --- | --- |
| `0x01` | Text | Exact UTF-8 bytes, including any embedded NUL; length 0–16,384 bytes. |
| `0x02` | Int | Exactly 8 bytes: signed i64 in two's complement. |
| `0x03` | Bool | Exactly 1 byte: `00` false or `01` true. |
| `0x10` | Text | `count:u16`, followed by exactly `count` Text register references (`u16` each). Concatenate in that order. |
| `0x11` | Int | Exactly two Int register references: `left:u16`, `right:u16`. Checked signed addition. |
| `0x12` | Text | Exactly one Text, Int, or Bool register reference (`u16`). Explicit text conversion. |
| `0x20` | Json | `field_count:u16`, followed by the object fields described below. |

Text has explicit byte lengths, with no terminator. Int literals preserve all values from −9223372036854775808 through 9223372036854775807. For example, −1 is `ff ff ff ff ff ff ff ff`. Bool accepts no values other than 0 and 1. Int conversion uses signed decimal text; Bool conversion produces `true` or `false`; Text conversion preserves the text. Json values cannot be converted with `0x12`.

For each object field, encode:

```text
key_length:u16 | key:UTF-8 bytes | value_register:u16
```

Keys must be unique and **strictly ascending in UTF-8 byte order**. Empty and embedded-NUL keys are permitted. Referenced values can have any of the four types; types remain intact in the resulting JSON. Empty objects are valid. Field order in the binary is canonical and determines size-error evaluation order.

The payload length must cover exactly the opcode's payload. Extra bytes, omitted operands, invalid UTF-8, duplicate/unsorted keys, invalid references, and incorrect result tags are errors. Counts are bounded by the payload itself before allocation or traversal.

## Entry payloads

The header's entry kind selects one of two forms, immediately after the last instruction.

### Console: entry kind 1

```text
value_register:u16
```

The referenced value must be Text. Execute all instructions, then write its exact bytes and one newline. No console output occurs before language evaluation succeeds. The complete text plus appended newline may contain at most 65,536 bytes.

### HTTP: entry kind 2

```text
port:u16 | route_count:u16 | routes...
```

Each route is:

```text
path_length:u16 | path:ASCII bytes | status:u16 | body_register:u16
```

- Port is 0–65535. Port 0 requests an available port.
- At most 32 routes are permitted. Zero routes is valid.
- GET is implicit; no textual method name is encoded.
- Paths are unique, start with `/`, and contain at most 2,048 ASCII bytes. Whitespace, controls, `?`, and `#` are invalid.
- Status is 200–299 except 204 and 205, which cannot carry the required JSON body.
- Body references must have type Json.

Native runtime support binds IPv4 loopback and serves the program's status/body with `application/json; charset=utf-8`. Query strings do not affect route matching; missing routes return 404 and unsupported methods return 405. Route bodies are computed once at startup. No request-dependent language values exist in RVB1.

## Evaluation and validation

Decoding checks file/header lengths, version/flags, record tags, exact payload shape, UTF-8, register order/types, object ordering, entry definitions, and static limits before accepting a program. It creates the private typed IR directly; it does not parse or generate a JSON source document, execute the reference interpreter, or evaluate the program to obtain output.

Instructions have straight-line, single-assignment semantics and execute once in source order, including instructions unused by the entry. Checked arithmetic overflow and computed-value bounds are execution errors. Compiler optimization must preserve their observable order. Inspection and native compilation can succeed for a structurally valid program that later fails at execution.

Resource limits:

| Resource | Limit |
| --- | ---: |
| Whole source | 65,536 bytes |
| Instruction records | 256 |
| Text literal | 16,384 bytes |
| Computed Text or serialized Json value | 65,536 bytes |
| Console output including newline | 65,536 bytes |
| HTTP routes | 32 |
| HTTP path | 2,048 ASCII bytes |

Malformed source reports the first error with an absolute offset such as `byte 0x0011` and a useful reason. Runtime overflow diagnostics use generated register labels such as `r3`; source-level symbolic names do not exist in the binary. Per-value bounds are not an aggregate hostile-program memory quota.

## Authoring a complete source file directly

This 35-byte file defines a single Text instruction and a console entry:

```text
52 56 42 00 01 00 00 00 01 00 01 00 13 00 00 00
01 01 0d 00 48 65 6c 6c 6f 2c 20 77 6f 72 6c 64
21 00 00
```

The first 16 bytes declare one instruction, console entry, and 19 body bytes. At offset `0x10`, opcode `01` and type `01` precede a 13-byte literal. The literal occupies offsets `0x14`–`0x20`; the final two bytes select register 0 for output.

An AI author can write those bytes with any binary-capable file tool. For example, this byte-writing command creates the source directly without a textual language frontend:

```sh
python3 - <<'PY'
from pathlib import Path
Path("hello.rvb").write_bytes(bytes.fromhex(
    "52 56 42 00 01 00 00 00 01 00 01 00 13 00 00 00 "
    "01 01 0d 00 48 65 6c 6c 6f 2c 20 77 6f 72 6c 64 "
    "21 00 00"
))
PY
```

The resulting `hello.rvb` is the source. The Python snippet merely writes bytes; Python is not used to implement or run the Hello program. Hex here is documentation/file-writing notation, not another source format accepted by the compiler.

With Rivet on PATH:

```sh
rivet inspect hello.rvb
rivet build hello.rvb -o hello
./hello
```

The program prints `Hello, world!` followed by a newline. The compiler produces the same standalone native artifact model described in [Architecture](ARCHITECTURE.md).

## Editing and inspection

The workbench keeps source as byte arrays, opens and saves actual `.rvb` files, and obtains human-readable views from the decoder. An instruction's `offset`, `payload_offset`, and `payload_length` identify the source bytes underlying its decoded view. Scalar literals are exposed exactly as strings in inspection metadata, including decimal i64 values beyond JavaScript's safe integer range. This metadata is not canonical program source.

Changing a variable-length payload requires updating its u16 record length and the header's u32 body length. Inserting/removing an instruction also changes its ordinal and may require updating subsequent references, the entry reference, and header instruction count. Hex editing permits complete low-level changes; decoded literal controls provide convenient byte-preserving edits to common fields. Invalid intermediate bytes are not accepted for execution.

Every accepted RVB1 file has a deterministic byte-exact decode/encode round trip. Reserved fields, exact lengths, canonical booleans, and sorted object keys eliminate alternate encodings for the same record structure. Program debug names from the legacy JSON prototype are intentionally absent. An optional library encoder operates on already validated typed IR for tests/migration; authoring `.rvb` directly does not require it.

## Scope

RVB1 implements the existing bounded computation semantics with a binary source model. It adds no loops, user branches, functions, mutable state, arrays, null values, files, subprocesses, or external network operations. LLVM/Clang remains the machine-code backend. Binary source is not an executable CPU instruction stream, and its compact form does not by itself establish better model-generation reliability, token efficiency, or runtime performance.
