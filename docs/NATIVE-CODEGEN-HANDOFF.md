# Native compiler lowering handoff

Updated 2026-10-25. Implements the compiler-owned scope in the committed
`NATIVE-PLAN.md` (`bddf7f0`), after reading the project instructions, brief,
architecture, state, contract, and prior validation/review records.

## Delivered files and interfaces

- `src/language.rs`: adds public `EntryInfo` (`Clone + Debug + Serialize`,
  internally tagged lowercase `kind`) and `CompiledProgram::entry_info()` /
  `CompiledProgram::emit_llvm()` with the exact planned signatures. The reference
  parser, validator, interpreter, and disassembly are unchanged.
- `src/language/native_codegen.rs`: directly lowers private typed register
  instructions into deterministic opaque-pointer LLVM IR.
- `tests/native_codegen.rs`: eight tests covering emitted code, actual Clang
  acceptance, and reference/native differential execution for both profiles.

No Git writes, shared-doc changes, runtime/backend/UI edits, source evaluation,
or dependency changes were made by this owner. Coordinator owns integration,
commits, complete acceptance, and publication.

## Lowering and semantic decisions

Every potentially failing source instruction is emitted in source order,
including unused values. Checked addition uses
`llvm.sadd.with.overflow.i64`, extracts the scalar/overflow results, and branches
to an instruction-specific `rv_fail` call or the next instruction. No unchecked
integer add helper or interpreter dispatch exists in the generated program.

Integer and boolean literals become i64/i1 constants. Boolean arguments are
explicitly zero-extended to the C ABI's unsigned byte. Text/JSON operations use
the fixed native support ABI. Text-to-text conversion aliases immutable storage;
this is semantically equivalent to the reference copy for valid bounded text.
Nested Json values are passed directly, while primitive object values use the
typed JSON support constructors. Keys are already sorted by the validated IR.

All literal/key/path/diagnostic bytes are hexadecimal escaped, have an additional
NUL terminator, and retain an explicit original byte length. Embedded NUL,
Unicode, quotes, backslashes, and newlines cannot become LLVM syntax. Program
names are not interpolated into code or comments; instruction IDs are validated
ASCII identifiers. Empty arrays use null pointers and zero counts. Runtime
pointer arrays use stack storage; bytes/key metadata use immutable globals.

Console entry calls `rv_print`. HTTP entry registers program-defined paths,
status codes, and computed bodies before calling `rv_serve`. Normal exits call
`rv_cleanup` and return the service status (or zero for console).

Runtime coordination confirmed JSON text wrappers allow temporary escaped
encodings larger than 64 KiB so `rv_object` can enforce computed-Json bounds in
field order. This avoids changing which size error occurs when a preceding
field already exceeds the object's bound. Allocation failures remain native
support errors, as specified in the ABI.

Overflow diagnostics identify the source instruction and register but omit the
reference interpreter's dynamic operand values. Other runtime diagnostics are
provided by C support and have no instruction prefix under ABI v1. The
coordinator accepted this wording difference: failure category, ordering, exit
failure, and absence of partial console output are preserved.

## Validation actually performed

Environment: Apple Clang 17.0.0, arm64-apple-darwin25.5.0; pinned project Rust.

| Command | Observed result |
| --- | --- |
| `cargo test --locked --test native_codegen --test language` | Passed all 8 native-codegen tests and all 24 unchanged reference-language tests. Native-codegen tests completed in 4.31 seconds. |
| `rustfmt --edition 2024 --check src/language.rs src/language/native_codegen.rs tests/native_codegen.rs` | Passed. |
| `cargo clippy --locked --test native_codegen -- -D warnings` | Passed. |

The native suite compiled five emitted modules into real object files using
Clang, covering all examples, empty values, and escaped UTF-8/NUL literals/keys.
It also built and ran 22 console executables through the coordinator's
`build_native`, testing both `Baseline` (-O0) and `Size` (-Oz) for each case:

- Exact Hello output; signed-i64 extrema and addition; true/false; immutable
  text aliasing; NUL/Unicode/escaping; empty concat; exact console-size boundary.
- Unused positive and negative overflow, with a later failing text operation;
  failing text followed by unused overflow, proving first-failure preservation.
- Console-newline overflow, exact Json-size acceptance, one-byte Json overflow,
  and Json escaping expansion beyond the bound.

Successful native stdout is compared byte-for-byte with reference execution;
failures must have a nonzero status, empty stdout, and the expected reference
error category. Shape tests verify that emitting metadata/LLVM does not execute
an overflowing program, and that route/entry cleanup lowering is present.

An earlier invocation briefly encountered a Cargo.lock mismatch during the
coordinator's concurrent dependency update; the recorded successful invocation
above ran afterward with `--locked`. No unresolved compiler test failures remain.

## Remaining integration checks

Coordinator still owns HTTP response differential tests (including exact i64,
sorted/escaped keys, nested JSON), copied-artifact independence, OS-process
lifecycle, browser verification, clean checkout, efficiency measurements, and
final whole-project validation. This handoff does not claim either native MVP
complete on compiler tests alone.
