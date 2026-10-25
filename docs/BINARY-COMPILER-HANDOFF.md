# RVB1 compiler handoff

Implemented 2026-10-25 under the ownership committed in `docs/BINARY-PLAN.md` (`372d143`). Only `src/language.rs`, `src/language/binary.rs`, `tests/binary_source.rs`, `docs/BINARY-FORMAT.md`, and this handoff were changed by this owner. No Git mutations or dependency changes were performed.

## Implementation

- `language::compile_binary(&[u8])` directly validates RVB1 bytes and constructs the existing private typed IR. It never calls the legacy JSON parser, produces JSON source, or evaluates program values while decoding.
- `language::inspect_binary(&[u8])` returns the agreed `BinaryInspection`: readable `ir`, per-instruction metadata, entry metadata, entry offset, and source byte count. Instruction metadata contains ordinal, numeric opcode, result type, record/payload offsets and payload length, plus an optional exact scalar literal string. Decimal i64 values are never routed through floating point.
- `CompiledProgram::encode_binary()` emits canonical bytes from validated typed IR. Debug names are deliberately omitted. Every successfully decoded binary source round-trips exactly. Legacy typed IR can be encoded for migration/testing without making that a prerequisite for binary authorship.
- `compile(&str)` is explicitly documented as the legacy JSON parser retained for historical semantic tests/migration utilities. Primary CLI/workbench use of `compile_binary` belongs to the coordinator.
- `BINARY-FORMAT.md` provides the complete header/record/entry grammar, byte order, opcode/type tables, typing/evaluation/resource rules, error behavior, inspection/editing contract, and a complete 35-byte Hello source with a direct byte-writing example.

No planned interface changed. The binary decoder assigns internal debug IDs `r0`, `r1`, etc. and names programs `Binary console program` or `Binary HTTP program`; these strings do not exist in source files. It reports the first structural/type problem as one `Diagnostic`, with `path` such as `byte 0x0011`. It enforces body/record lengths, exact end of file, header constants, type tags, backward references, valid UTF-8, canonical bools and sorted unique object keys, route shape/status/type/path/count, and existing source/literal/instruction limits. Reader slices and untrusted counts are bounded before traversal/allocation. Runtime overflow and computed-value errors remain separate from decoding.

## Validation actually performed

On the project macOS/Apple Silicon host:

| Check | Result |
| --- | --- |
| `cargo check --locked --lib` | Passed after adding the public interfaces. |
| `cargo test --locked --test binary_source` | All **13** tests passed, including six compiled native artifacts under both profiles; 2.67 seconds in this run. |
| `cargo clippy --locked --test binary_source -- -D warnings` | Passed. |
| `cargo test --locked --test language --test native_codegen` | All **24** unchanged semantic tests and **8** existing native-codegen tests passed. |
| `rustfmt --edition 2024 --check src/language.rs src/language/binary.rs tests/binary_source.rs` | Passed. |
| `git diff --check -- src/language.rs src/language/binary.rs tests/binary_source.rs docs/BINARY-FORMAT.md` | Passed. |

New tests author byte records directly, without JSON. Coverage includes:

- An explicit 35-byte golden source, exact Hello output, JSON rejection, native LLVM lowering, byte-exact encode/decode.
- Inspector offsets, NUL/Unicode, exact minimum-i64 literal string, boolean metadata, and typed decoded instructions.
- Unknown magic/version/flags/reserved/entry/opcode/type fields; inconsistent body and payload lengths; trailing bytes; every truncated prefix of an all-operation source with both original and repaired header lengths.
- Missing/self/forward/wrong-type operands, Json-to-Text rejection, malformed scalar widths, noncanonical bool values, impossible operand/field counts, invalid UTF-8, unsorted/duplicate object keys, and invalid object refs.
- HTTP path/status/ref/duplicate/count limits, exact 2,048-byte path, maximum port, empty HTTP program, exact 65,536-byte source, 16,384-byte literal boundary, and all 256 registers.
- **6,000** deterministic random/mutated inputs checked for no panics; every accepted mutation must re-encode byte-for-byte and inspect successfully.
- All seven operations, exact i64 extrema, safe arithmetic, false/true, text conversion/aliasing, nested JSON, escaped/NUL/Unicode keys/values, and deterministic byte roundtrip.
- Build/inspection do not execute a structurally valid overflow program. An unused overflow still fails during reference/native execution.
- Native baseline and size profiles both execute the direct-byte Hello with cleared environment and nonexistent PATH; both serve exact reference JSON bytes/status for an all-operation HTTP program; both reject negative checked overflow with nonzero exit and no partial stdout.

## Integration responsibilities and limits

The coordinator owns tracked directly authored `.rvb` examples, default CLI/build/read/inspect paths, legacy fixture relocation, raw-binary API transport, current shared documentation, full-suite/clean-checkout validation, actual GUI acceptance, and publication. The UI owner consumes the public inspection fields. This handoff does not claim complete project acceptance by itself.

No textual assembler, binary-to-JSON compiler stage, parser fallback, native emitter rewrite, new language operations, request-dependent execution, or AI-performance claim was introduced. Runtime resource semantics and generated-code ABI are unchanged. The byte format intentionally allows recognizable text literals; it is neither encrypted nor intrinsically inaccessible to humans.
