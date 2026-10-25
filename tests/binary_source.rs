use rivet::{
    language::{self, EntryInfo, Execution},
    native::{self, BuildProfile},
    native_process::NativeServer,
};
use std::{fs, path::PathBuf, process::Command};

// A complete source program written directly as bytes. No JSON/assembler input.
const HELLO: &[u8] = b"RVB\0\x01\0\0\0\x01\0\x01\0\x13\0\0\0\x01\x01\x0d\0Hello, world!\0\0";

fn record(opcode: u8, tag: u8, payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![opcode, tag];
    bytes.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}
fn refs(registers: &[u16]) -> Vec<u8> {
    registers.iter().flat_map(|r| r.to_le_bytes()).collect()
}
fn concat(registers: &[u16]) -> Vec<u8> {
    let mut bytes = (registers.len() as u16).to_le_bytes().to_vec();
    bytes.extend(refs(registers));
    record(0x10, 1, &bytes)
}
fn object(fields: &[(&str, u16)]) -> Vec<u8> {
    let mut bytes = (fields.len() as u16).to_le_bytes().to_vec();
    for (key, register) in fields {
        bytes.extend_from_slice(&(key.len() as u16).to_le_bytes());
        bytes.extend_from_slice(key.as_bytes());
        bytes.extend_from_slice(&register.to_le_bytes());
    }
    record(0x20, 4, &bytes)
}
fn http(port: u16, routes: &[(&str, u16, u16)]) -> Vec<u8> {
    let mut bytes = refs(&[port, routes.len() as u16]);
    for (path, status, body) in routes {
        bytes.extend_from_slice(&(path.len() as u16).to_le_bytes());
        bytes.extend_from_slice(path.as_bytes());
        bytes.extend(refs(&[*status, *body]));
    }
    bytes
}
fn program(records: &[Vec<u8>], entry_tag: u8, entry: &[u8]) -> Vec<u8> {
    let mut bytes = b"RVB\0\x01\0\0\0".to_vec();
    bytes.extend_from_slice(&(records.len() as u16).to_le_bytes());
    bytes.extend_from_slice(&[entry_tag, 0]);
    bytes.extend_from_slice(&0u32.to_le_bytes());
    for record in records {
        bytes.extend_from_slice(record);
    }
    bytes.extend_from_slice(entry);
    body_length(&mut bytes);
    bytes
}
fn body_length(bytes: &mut [u8]) {
    let length = (bytes.len() - 16) as u32;
    bytes[12..16].copy_from_slice(&length.to_le_bytes());
}
fn rejection(source: &[u8], expected: &str) {
    let diagnostics = language::compile_binary(source).unwrap_err();
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].path.starts_with("byte 0x"));
    assert!(
        diagnostics[0].message.contains(expected),
        "expected {expected:?}; got {:?}",
        diagnostics[0]
    );
    assert!(language::inspect_binary(source).is_err());
}
fn stdout(source: &[u8]) -> String {
    match language::compile_binary(source).unwrap().execute().unwrap() {
        Execution::Console { stdout } => stdout,
        _ => panic!("expected console"),
    }
}

#[test]
fn direct_bytes_are_source_and_decode_without_json() {
    assert_eq!(HELLO.len(), 35);
    assert_eq!(stdout(HELLO), "Hello, world!\n");
    let program = language::compile_binary(HELLO).unwrap();
    assert_eq!(program.encode_binary(), HELLO);
    assert_eq!(program.name(), "Binary console program");
    assert!(program.emit_llvm().contains("@rv_print"));
    rejection(
        br#"{"version":1,"instructions":[]}"#,
        "expected RVB1 binary source",
    );
}

#[test]
fn inspection_preserves_exact_offsets_and_scalar_literals() {
    let bytes = program(
        &[
            record(1, 1, "☃\0".as_bytes()),
            record(2, 2, &i64::MIN.to_le_bytes()),
            record(3, 3, &[1]),
            record(0x12, 1, &refs(&[1])),
        ],
        1,
        &refs(&[3]),
    );
    let view = language::inspect_binary(&bytes).unwrap();
    assert_eq!(view.source_bytes, bytes.len());
    assert_eq!(view.entry_offset, bytes.len() - 2);
    assert!(matches!(view.entry, EntryInfo::Console));
    assert_eq!(view.instructions.len(), 4);
    assert_eq!(view.instructions[0].offset, 16);
    assert_eq!(view.instructions[0].payload_offset, 20);
    assert_eq!(view.instructions[0].payload_length, 4);
    assert_eq!(view.instructions[0].literal.as_deref(), Some("☃\0"));
    assert_eq!(view.instructions[1].offset, 24);
    assert_eq!(
        view.instructions[1].literal.as_deref(),
        Some("-9223372036854775808")
    );
    assert_eq!(view.instructions[2].type_name, "Bool");
    assert_eq!(view.instructions[2].literal.as_deref(), Some("true"));
    assert_eq!(view.instructions[3].literal, None);
    assert!(view.ir.contains("%3: Text = to_text %1"));
    assert_eq!(stdout(&bytes), "-9223372036854775808\n");
}

#[test]
fn rejects_invalid_header_fields_and_exact_length_mismatch() {
    for (offset, value, reason) in [
        (0, b'X', "expected RVB1"),
        (4, 2, "version"),
        (6, 1, "flags"),
        (10, 0, "entry tag"),
        (11, 1, "reserved"),
        (12, 0, "body length"),
    ] {
        let mut bytes = HELLO.to_vec();
        bytes[offset] = value;
        rejection(&bytes, reason);
    }
    let mut bytes = HELLO.to_vec();
    bytes[8..10].copy_from_slice(&257u16.to_le_bytes());
    rejection(&bytes, "256 instructions");
    bytes = HELLO.to_vec();
    bytes[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
    rejection(&bytes, "body length");
    bytes = HELLO.to_vec();
    bytes.push(0);
    rejection(&bytes, "body length");
    body_length(&mut bytes);
    rejection(&bytes, "trailing bytes");
}

#[test]
fn every_truncated_prefix_fails_without_panicking() {
    let source = all_operations();
    for end in 0..source.len() {
        let mut bytes = source[..end].to_vec();
        assert!(language::compile_binary(&bytes).is_err(), "prefix {end}");
        if end >= 16 {
            body_length(&mut bytes);
            assert!(
                language::compile_binary(&bytes).is_err(),
                "consistent-length prefix {end}"
            );
        }
    }
}

#[test]
fn rejects_bad_opcodes_types_payloads_and_utf8() {
    let cases = [
        (record(0xff, 1, b""), "unknown opcode"),
        (record(1, 2, b""), "result type tag"),
        (record(1, 0xff, b""), "result type tag"),
        (record(1, 1, &[0xff]), "UTF-8"),
        (record(2, 2, &[0; 7]), "truncated Int literal"),
        (record(2, 2, &[0; 9]), "trailing bytes"),
        (record(3, 3, &[2]), "Bool literal must be 0 or 1"),
        (record(3, 3, &[1, 0]), "trailing bytes"),
        (record(0x10, 1, &[0xff, 0xff]), "operand count exceeds"),
        (record(0x20, 4, &[0xff, 0xff]), "field count exceeds"),
    ];
    for (record, reason) in cases {
        rejection(&program(&[record], 1, &refs(&[0])), reason);
    }
    let mut bytes = HELLO.to_vec();
    bytes[18..20].copy_from_slice(&65535u16.to_le_bytes());
    rejection(&bytes, "truncated instruction payload");
}

#[test]
fn typed_references_reject_self_forward_missing_and_wrong_types() {
    rejection(
        &program(&[record(0x12, 1, &refs(&[0]))], 1, &refs(&[0])),
        "not defined",
    );
    rejection(
        &program(&[concat(&[1]), record(1, 1, b"later")], 1, &refs(&[0])),
        "not defined",
    );
    rejection(
        &program(&[record(1, 1, b"text"), concat(&[65535])], 1, &refs(&[1])),
        "not defined",
    );
    rejection(
        &program(
            &[record(2, 2, &1i64.to_le_bytes()), concat(&[0])],
            1,
            &refs(&[1]),
        ),
        "expected Text, found Int",
    );
    rejection(
        &program(
            &[record(1, 1, b"text"), record(0x11, 2, &refs(&[0, 0]))],
            1,
            &refs(&[0]),
        ),
        "expected Int, found Text",
    );
    rejection(
        &program(&[object(&[]), record(0x12, 1, &refs(&[0]))], 1, &refs(&[1])),
        "found Json",
    );
    rejection(
        &program(&[record(2, 2, &1i64.to_le_bytes())], 1, &refs(&[0])),
        "expected Text, found Int",
    );
    rejection(&program(&[], 1, &refs(&[0])), "not defined");
}

#[test]
fn object_keys_require_unique_sorted_utf8_and_valid_operands() {
    for fields in [[("z", 0), ("a", 0)], [("a", 0), ("a", 0)]] {
        rejection(
            &program(&[record(1, 1, b"value"), object(&fields)], 1, &refs(&[0])),
            "strictly ascending",
        );
    }
    rejection(
        &program(
            &[record(1, 1, b"value"), object(&[("a", 9)])],
            1,
            &refs(&[0]),
        ),
        "not defined",
    );
    let invalid_key = record(0x20, 4, &[1, 0, 1, 0, 0xff, 0, 0]);
    rejection(
        &program(&[record(1, 1, b"value"), invalid_key], 1, &refs(&[0])),
        "UTF-8 in object key",
    );
    let bytes = program(
        &[
            record(1, 1, b"value"),
            object(&[("", 0), ("a\0", 0), ("é", 0)]),
        ],
        2,
        &http(0, &[("/", 200, 1)]),
    );
    assert_eq!(
        language::compile_binary(&bytes).unwrap().encode_binary(),
        bytes
    );
}

#[test]
fn validates_http_fields_limits_and_route_references() {
    let instructions = [object(&[])];
    for path in [
        "",
        "hello",
        "/space here",
        "/?query",
        "/#fragment",
        "/é",
        "/\0",
    ] {
        rejection(
            &program(&instructions, 2, &http(0, &[(path, 200, 0)])),
            "route path must start",
        );
    }
    for status in [0, 199, 204, 205, 300, 65535] {
        rejection(
            &program(&instructions, 2, &http(0, &[("/", status, 0)])),
            "status must be",
        );
    }
    rejection(
        &program(&instructions, 2, &http(0, &[("/", 200, 0), ("/", 201, 0)])),
        "duplicate route",
    );
    rejection(
        &program(&[record(1, 1, b"text")], 2, &http(0, &[("/", 200, 0)])),
        "expected Json, found Text",
    );
    rejection(
        &program(&instructions, 2, &http(0, &[("/", 200, 99)])),
        "not defined",
    );
    rejection(
        &program(&instructions, 2, &refs(&[0, 33])),
        "32 HTTP routes",
    );
    let long_path = format!("/{}", "a".repeat(2048));
    rejection(
        &program(&instructions, 2, &http(0, &[(&long_path, 200, 0)])),
        "2048 bytes",
    );
    let exact_path = &long_path[..2048];
    let bytes = program(&instructions, 2, &http(65535, &[(exact_path, 299, 0)]));
    assert_eq!(
        language::compile_binary(&bytes).unwrap().encode_binary(),
        bytes
    );
    let empty = program(&[], 2, &http(0, &[]));
    assert_eq!(
        language::compile_binary(&empty).unwrap().encode_binary(),
        empty
    );
}

#[test]
fn source_literal_and_instruction_bounds_are_enforced() {
    rejection(&vec![0; 65537], "source exceeds");
    rejection(
        &program(&[record(1, 1, &vec![b'x'; 16385])], 1, &refs(&[0])),
        "literal exceeds",
    );
    let exact = program(
        &[
            record(1, 1, &vec![b'a'; 16384]),
            record(1, 1, &vec![b'b'; 16384]),
            record(1, 1, &vec![b'c'; 16384]),
            record(1, 1, &vec![b'd'; 16350]),
        ],
        1,
        &refs(&[3]),
    );
    assert_eq!(exact.len(), 65536);
    assert_eq!(
        language::compile_binary(&exact).unwrap().encode_binary(),
        exact
    );
    let records = vec![record(1, 1, b""); 256];
    let bytes = program(&records, 1, &refs(&[255]));
    assert_eq!(
        language::compile_binary(&bytes)
            .unwrap()
            .instruction_count(),
        256
    );
    assert_eq!(stdout(&bytes), "\n");
}

fn all_operations() -> Vec<u8> {
    program(
        &[
            record(1, 1, "UTF-8 ☃\0\n\"\\".as_bytes()), // 0
            record(2, 2, &i64::MAX.to_le_bytes()),      // 1
            record(2, 2, &i64::MIN.to_le_bytes()),      // 2
            record(3, 3, &[1]),                         // 3
            record(3, 3, &[0]),                         // 4
            record(2, 2, &40i64.to_le_bytes()),         // 5
            record(2, 2, &2i64.to_le_bytes()),          // 6
            record(0x11, 2, &refs(&[5, 6])),            // 7
            record(0x12, 1, &refs(&[7])),               // 8
            concat(&[0, 8]),                            // 9
            record(0x12, 1, &refs(&[3])),               // 10
            record(0x12, 1, &refs(&[0])),               // 11
            object(&[
                ("false", 4),
                ("max", 1),
                ("min", 2),
                ("sum", 7),
                ("text", 9),
                ("truth", 10),
            ]), // 12
            object(&[("a\0", 11), ("nested", 12), ("true", 3)]), // 13
        ],
        2,
        &http(0, &[("/values", 201, 13)]),
    )
}

#[test]
fn all_operations_roundtrip_and_reference_preserves_exact_values() {
    let bytes = all_operations();
    let compiled = language::compile_binary(&bytes).unwrap();
    assert_eq!(compiled.encode_binary(), bytes);
    let Execution::Http { port, routes } = compiled.execute().unwrap() else {
        panic!()
    };
    assert_eq!(port, 0);
    assert_eq!(routes[0].body["nested"]["max"], i64::MAX);
    assert_eq!(routes[0].body["nested"]["min"], i64::MIN);
    assert_eq!(routes[0].body["nested"]["sum"], 42);
    assert_eq!(routes[0].body["nested"]["truth"], "true");
    assert_eq!(routes[0].body["a\0"], "UTF-8 ☃\0\n\"\\");
    assert!(matches!(
        language::inspect_binary(&bytes).unwrap().entry,
        EntryInfo::Http { port: 0, .. }
    ));
}

#[test]
fn inspection_does_not_execute_and_unused_overflow_still_fails_at_runtime() {
    let bytes = program(
        &[
            record(1, 1, b"unreached"),
            record(2, 2, &i64::MAX.to_le_bytes()),
            record(2, 2, &1i64.to_le_bytes()),
            record(0x11, 2, &refs(&[1, 2])),
        ],
        1,
        &refs(&[0]),
    );
    assert!(language::inspect_binary(&bytes).is_ok());
    let compiled = language::compile_binary(&bytes).unwrap();
    assert!(compiled.execute().unwrap_err().contains("overflow"));
    assert!(compiled.emit_llvm().contains("llvm.sadd.with.overflow.i64"));
    assert_eq!(compiled.encode_binary(), bytes);
}

#[test]
fn random_and_mutated_inputs_never_panic_and_valid_bytes_are_canonical() {
    let original = all_operations();
    let mut seed = 0xa13e_7105_cab1_4455u64;
    let mut random = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for iteration in 0..6000 {
        let mut bytes = if iteration % 2 == 0 {
            original.clone()
        } else {
            let length = (random() % 512) as usize;
            (0..length).map(|_| random() as u8).collect()
        };
        for _ in 0..1 + (random() % 8) {
            if !bytes.is_empty() {
                let index = (random() as usize) % bytes.len();
                bytes[index] = random() as u8;
            }
        }
        let outcome = std::panic::catch_unwind(|| language::compile_binary(&bytes));
        assert!(outcome.is_ok(), "panic at mutation {iteration}");
        if let Ok(program) = outcome.unwrap() {
            assert_eq!(
                program.encode_binary(),
                bytes,
                "noncanonical input {iteration}"
            );
            assert!(language::inspect_binary(&bytes).is_ok());
        }
    }
}

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(native::unique_directory(&std::env::temp_dir(), "rivet-binary-test").unwrap())
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn binary_source_native_console_http_and_overflow_match_reference_both_profiles() {
    let temp = Temp::new();
    let hello = language::compile_binary(HELLO).unwrap();
    let http_program = language::compile_binary(&all_operations()).unwrap();
    let Execution::Http { routes, .. } = http_program.execute().unwrap() else {
        panic!()
    };
    let expected = routes[0].body.to_string();
    let overflow = language::compile_binary(&program(
        &[
            record(1, 1, b"should not print"),
            record(2, 2, &i64::MIN.to_le_bytes()),
            record(2, 2, &(-1i64).to_le_bytes()),
            record(0x11, 2, &refs(&[1, 2])),
        ],
        1,
        &refs(&[0]),
    ))
    .unwrap();
    for profile in [BuildProfile::Baseline, BuildProfile::Size] {
        let artifact = native::build_native(
            &hello,
            &temp.0.join(format!("hello-{}", profile.name())),
            profile,
        )
        .unwrap();
        let output = Command::new(&artifact.path)
            .env_clear()
            .env("PATH", "/nonexistent")
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"Hello, world!\n");
        assert!(output.stderr.is_empty());
        let artifact = native::build_native(
            &http_program,
            &temp.0.join(format!("http-{}", profile.name())),
            profile,
        )
        .unwrap();
        let mut server = NativeServer::start(&artifact).unwrap();
        let response = server.request("/values?binary=1").unwrap();
        assert_eq!(response.status, 201);
        assert_eq!(response.body, expected);
        server.stop().unwrap();
        let artifact = native::build_native(
            &overflow,
            &temp.0.join(format!("overflow-{}", profile.name())),
            profile,
        )
        .unwrap();
        let output = Command::new(&artifact.path).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("overflow"));
    }
}
