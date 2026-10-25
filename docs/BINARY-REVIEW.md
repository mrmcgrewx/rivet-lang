# Independent RVB1 review

Reviewed 2026-10-25 against the binary-source contract committed in `372d143` ([BINARY-PLAN.md](BINARY-PLAN.md)). The reviewer changed only this document and made no production, test-suite, or Git mutations. Checks below used the integrated working tree while the coordinator and assigned owners completed their contributions.

## Architectural result

The primary source path is binary. Tracked `examples/*.rvb` contain an RVB1 header, numeric opcode/type/length records, register indices, literal payloads, and a binary entry. The example writer authors these bytes directly and does not read or convert JSON programs. The source files are distinct from the native Mach-O executables built from them.

`src/language/binary.rs` validates the byte stream and directly constructs the private typed IR. It does not call the legacy JSON parser, translate through a JSON document, or evaluate the program to precompute executable output. The CLI reads bounded bytes and calls `compile_binary`; workbench Build/Inspect accept raw `application/octet-stream` bodies and call the binary decoder. JSON control messages and inspection responses remain API data, not program source. The old parser remains only as an explicitly labelled library facility used by historical tests/migration.

The unchanged LLVM emitter consumes the validated IR and emits native operations/direct C support calls. Source files, readable inspection, compiler implementation, and generated executables remain separate. A binary source is not claimed to be CPU machine code, encrypted, or inherently impossible for humans to decode. Recognizable literal text is accurately documented.

## Decoder and encoding review

Reviewed exact header/body and record lengths, reserved flags/version/tags, scalar widths, canonical booleans, UTF-8, opcode/result consistency, backward-only typed operands, sorted unique object keys, HTTP entry shape, path/status/ref constraints, and static bounds. Reads are bounded before slicing, and untrusted counts are checked against remaining payload bytes before allocation/traversal. Register references resolve through the already decoded instruction list. Extra/truncated bytes are rejected with absolute byte-offset diagnostics.

The encoder omits source/debug names and preserves every accepted binary structure byte-for-byte. Inspection exposes exact record/payload offsets and decimal i64 values as strings. Decoding and inspection do not execute overflow or computed-size checks prematurely; those remain runtime behavior, including for unused instructions.

## Workbench source-state review

The frontend stores canonical source in `Uint8Array`. File import uses `arrayBuffer`; binary export creates an octet-stream Blob from those bytes. The decoded text is a read-only view and is never downloaded or submitted as source. Literal editing splices payload bytes and updates record/body lengths; signed integers use BigInt/DataView rather than JavaScript Number. HTTP port edits modify the binary entry. Raw hex editing allows changes beyond the convenience controls.

Invalid hex syntax sets `state.bytes` to null, disabling Build, Run of a stale build, and binary-source export. Syntactically valid but structurally invalid bytes remain the actual current source and are submitted/exported unchanged; they cannot fall back to the previous accepted bytes. Existing native servers retain their own build until Stop.

Inspection requests carry a monotonically increasing source revision. Success, error, and finalization paths check that revision before updating inspection state; responses never overwrite canonical source bytes. The reviewer specifically exercised delayed older successful and failed inspections after newer edits, as described below.

## Finding and resolution

**Resolved: incorrect hex gutter offsets after custom line wrapping.** The initial GUI assumed 16 bytes per hex row although the parser accepted arbitrary whitespace. Pasting eight-byte rows therefore labelled the second row `0010` instead of `0008`, misleading byte-oriented editing.

The UI owner changed gutter calculation to count preceding hex digits and added a browser regression that pastes eight-byte rows and expects `0000`, `0008`, `0010`. The reviewer inspected the fix and regression. The UI owner reported the complete 12-scenario browser suite passing afterward. This finding is closed.

## Independent checks actually performed

| Check | Result |
| --- | --- |
| `cargo test --locked --test workbench --test native_artifacts --test native_codegen` | **17 tests passed**: 4 workbench, 5 native artifact/lifecycle, 8 native-codegen. |
| `cargo test --locked --test binary_source` | **13 tests passed**, including directly authored binary sources, malformed input, exact bounds/roundtrips, and native baseline/size differential checks. |
| Separate Rust mutation corpus | **99,915** malformed/mutated/truncated inputs completed without a panic; **15,434** accepted variants re-encoded byte-for-byte and produced valid inspection offsets. |
| Separate directly authored binary/native differential experiment | **12** generated programs, **24** native builds across baseline/size, all matching repeated reference HTTP bytes and exiting cleanly on SIGTERM. |
| Separate Chromium source-state probes | **3 tests passed** in 4.8 seconds against an independent workbench on an ephemeral port. |

The Rust mutation corpus enumerated every possible one-byte replacement at every byte position of all three committed examples, every truncated prefix, 10,000 seeded four-byte mutations per example, and boundary-size empty data. This is bounded fuzz-style evidence, not exhaustive proof.

The direct-binary experiment used deterministic seed `20261025` and wrote opcode records with little-endian integer packing, without a JSON source/frontend. Every program exercised all seven operations, safe signed addition, both boolean values, text conversion/concatenation, nested objects, i64 extrema, empty/Unicode/NUL/escaped literals and keys, and program-defined HTTP status/path. Each native profile was compared byte-for-byte to the binary-source reference interpreter over two real HTTP requests. All experiment processes were stopped; temporary executable/source artifacts were removed.

The independent browser probes used real backend inspection/build responses, delaying only the delivery of selected older inspection responses:

1. Build a valid source, then enter incomplete hex. Build, Run, and source export become disabled; changing views preserves the incomplete draft. Enter three malformed bytes instead, verify exported bytes are exactly those three bytes, and verify Build submits those bytes and fails rather than using the previous source.
2. Hold an older successful inspection response, edit newer source bytes, let the newer inspection finish, then release the older response. Decoded text and exported bytes remain the newer program; Build submits the newer bytes and its native output matches them.
3. Repeat with an older failed inspection. Its late error does not replace the newer valid inspection or diagnostics, and the newer program still builds/runs correctly.

These review probes were temporary files outside the repository and did not change the assigned browser suite. The reviewer also read that suite's binary open/save, raw build body, native artifact download, i64/boolean/text edits, HTTP lifecycle, runtime errors, and compact-layout coverage. The UI owner's full suite and the coordinator's actual in-app browser acceptance are separate evidence.

## Final review status

No unresolved implementation blocker was found after the gutter correction. The reviewed compiler, CLI/API, file state, binary edits, and native execution satisfy the binary-source architectural contract and the tested semantic/state boundaries.

The coordinator remains responsible for current shared documentation, final combined checks, actual in-app browser evidence, clean-checkout reproduction, and publication. Existing native runtime limitations, per-value rather than aggregate memory bounds, Apple Silicon verification scope, and lack of measured AI-generation advantage remain explicit limits; binary source does not remove them.
