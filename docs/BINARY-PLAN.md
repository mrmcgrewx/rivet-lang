# Binary source correction — RVB1

The user requires actual binary files as canonical authored source. JSON source is superseded. Native output alone did not meet this requirement. Build, run, check, workbench open/save, examples and documentation must center on `.rvb` binary sources. Do not hide JSON inside an envelope, rename JSON files, or require JSON authoring before encoding. AI authors can write bytes directly from the format specification. Human views are decoded inspection, not canonical source. Literal bytes may remain recognizable; binary encoding is not encryption or exclusively understandable by AI.

## Fixed RVB1 encoding

All multibyte integers are little-endian. Entire source <=65536 bytes. Numeric register IDs are zero-based record positions (max256); references must point backward. No names, textual opcodes, symbol table, or embedded source. Header is 16 bytes:

| Offset | Encoding | Meaning |
|---|---|---|
|0|4 bytes `52 56 42 00`|magic RVB NUL|
|4|u16 =1|version|
|6|u16 =0|flags, reject others|
|8|u16|instruction count|
|10|u8|entry tag 1 console, 2 HTTP|
|11|u8 =0|reserved|
|12|u32|remaining body byte length, exact EOF required|

Each instruction: opcode u8, result type u8, payload byte length u16, payload. Type tags 1 Text, 2 Int, 3 Bool, 4 Json. Reject mismatched tags, unknown opcodes, noncanonical bools, lengths/truncation/trailing bytes, bad refs/types/UTF8/limits.

| Opcode | Result | Payload |
|---|---|---|
|01|Text|raw UTF8, <=16384 bytes|
|02|Int|exactly8 bytes signed i64 two's complement|
|03|Bool|one byte 0 or1|
|10|Text|u16 operand count, followed by u16 Text register refs|
|11|Int|u16 left, u16 right Int refs; checked add|
|12|Text|u16 Text/Int/Bool ref; conversion|
|20|Json|u16 field count; each u16 key byte length, UTF8 key bytes, u16 value ref; keys strictly ascending UTF8 byte order, unique|

After the declared records, console entry payload is one u16 Text ref. HTTP entry payload: u16 port, u16 route count; each route contains u16 ASCII path byte length, path bytes, u16 status, u16 Json ref. GET implicit. Same existing route/status/resource semantics. Exact end required. Header and record lengths make byte offsets inspectable. Decoder constructs existing private typed IR directly, never through JSON. Reference interpreter and native LLVM path consume this validated IR.

## Interfaces and ownership (commit before delegation)

Binary compiler agent owns `src/language.rs`, new `src/language/binary.rs`, `tests/binary_source.rs`, `docs/BINARY-FORMAT.md`, `docs/BINARY-COMPILER-HANDOFF.md` only. Expose via language:
- `compile_binary(source: &[u8]) -> Result<CompiledProgram, Vec<Diagnostic>>`.
- `inspect_binary(source: &[u8]) -> Result<BinaryInspection, Vec<Diagnostic>>`.
- `CompiledProgram::encode_binary() -> Vec<u8>` deterministic canonical encoder of typed IR (for tests/legacy migration, not required for authoring).
- BinaryInspection Serialize fields `ir: String`, `instructions: Vec<BinaryInstructionInfo>`, `entry: EntryInfo`, `entry_offset: usize`, `source_bytes: usize`.
- BinaryInstructionInfo Serialize fields `index: usize`, `opcode: u8`, `type_name: String`, `offset: usize`, `payload_offset: usize`, `payload_length: usize`, `literal: Option<String>` (only scalar literals; exact decimal i64 and true/false text).
- Binary compiled name can be `Binary console program`/`Binary HTTP program`; internal debug IDs `r0`, etc. Byte diagnostics use `byte 0x...` paths and useful reason. Keep old `compile(&str)` only for legacy test coverage, explicitly documented as legacy. Never call it from binary decoder.

UI agent owns `web/`, `tests/ui/`, `docs/BINARY-UI-HANDOFF.md`. Canonical client state Uint8Array; binary file ArrayBuffer import and byte-exact Blob/octet-stream `.rvb` export. Readable decoded view + editable hex byte view, clearly labelled. No JSON program editor/codec. Backend inspection supplies readable IR, scalar literal edit offsets and entry offset. Literal editing can splice payload, update u16 record length and u32 body length; ints use BigInt, bool0/1; HTTP port edit uses entry_offset. Provide New binary source from fixed bytes, metadata-based example fetch, diagnostics and existing native controls. Hex allows arbitrary changes/create, inspector explains invalid binary. Preserve source/build/server separation, not text JSON parsing. Browser tests must cover binary import/export exactness, edits, invalid bytes, native artifacts and server lifecycle. No Rust/shared-doc edits.

Coordinator owns CLI/workbench, example generation, legacy fixture relocation, existing integration test adaptations, benchmarks, README/current docs/state, Git and combined validation. `examples/*.rvb` must be directly authored bytes (Python struct/bytes writer is a tool, not actual program source); prior JSON examples move to tests/fixtures/legacy. Main CLI commands load binary by default and reject JSON. `inspect` prints decoded source; existing check is alias validation+inspection. No default JSON migration path in CLI/UI.

Review agent may read all and owns only `docs/BINARY-REVIEW.md`; independently assess canonical bytes, decoder bounds/types, no hidden JSON/native fallback, editing roundtrip, and native independence. No production/Git mutations.

## Workbench protocol

`GET /api/examples`: metadata `{examples:[{id,name,url,filename}]}`; URL `/api/examples/<id>` serves raw binary bytes with application/octet-stream. Static examples embedded via include_bytes. `POST /api/inspect` and `POST /api/build`: raw binary body, Content-Type application/octet-stream, X-Rivet-Request:1; maintain local Host/Origin checks. Inspect returns `{ok:true,inspection: BinaryInspection}` or normal diagnostics. Build returns existing native BuildInfo, now from compile_binary only. Other run/stop/request JSON control envelopes remain unchanged; they are API commands, not source. Existing GET artifact download unchanged.

UI API fetch helper must support binary requests and separate binary GET. Hex display is an inspection/editor notation; save always actual bytes. Decoded source initially read-only with scalar/port edit controls (plus raw hex for full instruction editing). Build consumes exact current bytes. Preserve i64 values using BigInt/string.

## Acceptance

- Tracked binary Hello/server/computed examples built directly from opcodes; no public JSON source examples or required textual assembler/transcoder.
- CLI and workbench build native executable from raw binary, reject JSON on primary source path; standalone copies run after workbench exits.
- Native source roundtrip exact; malformed/truncated/header/count/type/ref/key/path/payload errors are bounded and useful, with adversarial/random input tests and both profiles differential execution.
- Actual GUI opens binary, shows decoded operations and hex, edits literal, exports correct binary, builds/runs changed behavior. HTTP native build/request/stop also works. Existing native code views/artifact lifecycle preserved.
- Current README/spec/state accurately present binary authorship and human decoder; record fresh validation and honest performance scope, no unsupported AI efficiency claim. Commit/push checkpoints; clean source checkout verification before claiming completion.
