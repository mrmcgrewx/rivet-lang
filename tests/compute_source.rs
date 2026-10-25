//! Direct-byte RVB2 tests. No text/JSON frontend participates in source creation.
use rivet::{
    language::{self, EntryInfo, Execution},
    native::{self, BuildProfile},
    native_process::run_console_with_input,
};
use std::{fs, path::PathBuf};

fn refs(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}
fn op(code: u8, tag: u8, values: &[u16]) -> Vec<u8> {
    record(code, tag, &refs(values))
}
fn record(code: u8, tag: u8, payload: &[u8]) -> Vec<u8> {
    let mut result = vec![code, tag];
    result.extend((payload.len() as u16).to_le_bytes());
    result.extend(payload);
    result
}
fn int(destination: u16, value: i64) -> Vec<u8> {
    let mut payload = refs(&[destination]);
    payload.extend(value.to_le_bytes());
    record(2, 2, &payload)
}
fn boolean(destination: u16, value: u8) -> Vec<u8> {
    let mut payload = refs(&[destination]);
    payload.push(value);
    record(3, 3, &payload)
}
fn array(destination: u16, values: &[u16]) -> Vec<u8> {
    let mut payload = refs(&[destination, values.len() as u16]);
    payload.extend(refs(values));
    record(0x34, 5, &payload)
}
fn program(types: &[u8], operations: &[Vec<u8>]) -> Vec<u8> {
    let mut result = b"RVB\0\x02\0\0\0".to_vec();
    result.extend((operations.len() as u16).to_le_bytes());
    result.extend([3, 0]);
    result.extend([0; 4]);
    result.extend((types.len() as u16).to_le_bytes());
    result.extend(types);
    for operation in operations {
        result.extend(operation);
    }
    result.extend([0; 2]);
    resize_header(&mut result);
    result
}
fn resize_header(source: &mut [u8]) {
    let length = source.len() as u32 - 16;
    source[12..16].copy_from_slice(&length.to_le_bytes());
}
fn reject(source: &[u8], reason: &str) {
    let errors = language::compile_binary(source).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].path.starts_with("byte 0x"));
    assert!(
        errors[0].message.contains(reason),
        "expected {reason}: {errors:?}"
    );
    assert!(language::inspect_binary(source).is_err());
}
fn output(source: &[u8], input: &str) -> Result<String, String> {
    match language::compile_binary(source)
        .unwrap()
        .execute_with_input(input)?
    {
        Execution::Console { stdout } => Ok(stdout),
        _ => panic!("compute must return console output"),
    }
}
fn echo_array() -> Vec<u8> {
    program(&[5], &[op(0x31, 5, &[0]), op(0x52, 0, &[0])])
}

#[test]
fn computation_roundtrip_and_inspection_do_not_consume_input() {
    let source = include_bytes!("../examples/two-sum.rvb");
    let program = language::compile_binary(source).unwrap();
    assert_eq!(program.encode_binary(), source);
    assert_eq!(program.instruction_count(), 21);
    assert!(matches!(program.entry_info(), EntryInfo::Compute));
    assert_eq!(
        serde_json::to_string(&program.entry_info()).unwrap(),
        r#"{"kind":"compute"}"#
    );
    assert!(program.execute().is_err());
    let inspection = language::inspect_binary(source).unwrap();
    assert_eq!(inspection.source_bytes, source.len());
    assert_eq!(inspection.entry_offset, source.len() - 2);
    assert!(inspection.ir.contains("@16: jump @7"));
    for info in inspection.instructions {
        assert!(info.literal.is_none());
        assert_eq!(source[info.offset], info.opcode);
        assert_eq!(info.payload_offset, info.offset + 4);
        assert_eq!(
            u16::from_le_bytes(source[info.offset + 2..info.offset + 4].try_into().unwrap())
                as usize,
            info.payload_length
        );
    }
    assert_eq!(output(source, "4 2 7 11 15 9").unwrap(), "[0,1]\n");
}

#[test]
fn exact_header_counts_types_and_lengths_are_required() {
    let valid = echo_array();
    for length in 0..valid.len() {
        assert!(
            language::compile_binary(&valid[..length]).is_err(),
            "prefix {length}"
        );
    }
    for (offset, value, reason) in [
        (6, 1, "flags"),
        (8, 0, "instructions"),
        (10, 1, "entry tag"),
        (11, 1, "reserved"),
        (16, 0, "registers"),
        (18, 4, "register type"),
    ] {
        let mut source = valid.clone();
        source[offset] = value;
        reject(&source, reason);
    }
    let mut source = valid.clone();
    source[8..10].copy_from_slice(&257u16.to_le_bytes());
    reject(&source, "instructions");
    let mut source = valid.clone();
    source[16..18].copy_from_slice(&257u16.to_le_bytes());
    reject(&source, "registers");
    let mut source = valid.clone();
    let tail = source.len() - 2;
    source[tail] = 1;
    reject(&source, "entry PC");
    let mut source = valid.clone();
    source.push(0);
    reject(&source, "body length");
    resize_header(&mut source);
    reject(&source, "trailing bytes");
    let mut source = vec![0; 65537];
    source[..6].copy_from_slice(b"RVB\0\x02\0");
    reject(&source, "65536");
}

#[test]
fn opcodes_tags_payloads_references_and_targets_are_checked() {
    for (bad, reason) in [
        (op(0xff, 0, &[]), "unknown RVB2 opcode"),
        (op(0x31, 2, &[0]), "result type tag"),
        (op(0x31, 5, &[1]), "outside the declared"),
        (op(0x30, 2, &[0]), "expected Int"),
        (op(0x31, 5, &[]), "truncated"),
        (op(0x31, 5, &[0, 0]), "trailing bytes"),
        (op(0x50, 0, &[2]), "branch target"),
        (op(0x34, 5, &[0, 10001]), "exceeds 10000"),
        (op(0x34, 5, &[0, 1]), "truncated array"),
    ] {
        reject(&program(&[5], &[bad, op(0x52, 0, &[0])]), reason);
    }
    reject(
        &program(&[3], &[boolean(0, 2), op(0x50, 0, &[0])]),
        "Bool literal",
    );
    reject(
        &program(&[5, 2], &[array(0, &[0]), op(0x52, 0, &[0])]),
        "expected Int",
    );
    reject(
        &program(&[3], &[boolean(0, 0), op(0x51, 0, &[0, 0, 2])]),
        "branch target",
    );
}

#[test]
fn definite_assignment_intersects_paths_and_checks_first_loop_iteration() {
    reject(
        &program(
            &[2, 3, 5],
            &[
                boolean(1, 1),
                op(0x51, 0, &[1, 2, 3]),
                int(0, 7),
                array(2, &[0]),
                op(0x52, 0, &[2]),
            ],
        ),
        "definitely initialized",
    );
    reject(
        &program(&[2], &[op(0x11, 2, &[0, 0, 0]), op(0x50, 0, &[0])]),
        "definitely initialized",
    );
    reject(
        &program(
            &[2],
            &[
                op(0x50, 0, &[2]),
                int(0, 1),
                op(0x11, 2, &[0, 0, 0]),
                op(0x50, 0, &[1]),
            ],
        ),
        "definitely initialized",
    );
    let diamond = program(
        &[2, 3, 5],
        &[
            boolean(1, 1),
            op(0x51, 0, &[1, 2, 4]),
            int(0, 7),
            op(0x50, 0, &[5]),
            int(0, 9),
            array(2, &[0]),
            op(0x52, 0, &[2]),
        ],
    );
    assert_eq!(output(&diamond, "").unwrap(), "[7]\n");
    // A backward edge is legal after initialization, including an in-place result.
    let loop_program = program(
        &[2, 2, 2, 3, 5],
        &[
            int(0, 0),
            int(1, 1),
            int(2, 3),
            op(0x15, 3, &[3, 0, 2]),
            op(0x51, 0, &[3, 5, 7]),
            op(0x11, 2, &[0, 0, 1]),
            op(0x50, 0, &[3]),
            array(4, &[0]),
            op(0x52, 0, &[4]),
        ],
    );
    assert_eq!(output(&loop_program, "").unwrap(), "[3]\n");
}

#[test]
fn reachable_fallthrough_is_rejected_but_valid_unreachable_tail_is_allowed() {
    reject(&program(&[2], &[int(0, 1)]), "falls through");
    reject(
        &program(
            &[3],
            &[boolean(0, 1), op(0x51, 0, &[0, 0, 2]), boolean(0, 0)],
        ),
        "falls through",
    );
    let source = program(
        &[5, 2],
        &[array(0, &[]), op(0x52, 0, &[0]), op(0x11, 2, &[1, 1, 1])],
    );
    assert_eq!(output(&source, "").unwrap(), "[]\n");
    assert!(
        language::compile_binary(&source)
            .unwrap()
            .emit_llvm()
            .contains("unreachable")
    );
    // Unreachable records still have to be structurally and statically well typed.
    reject(
        &program(
            &[5, 2],
            &[array(0, &[]), op(0x52, 0, &[0]), op(0x11, 2, &[1, 0, 1])],
        ),
        "expected Int",
    );
}

#[test]
fn input_and_array_semantics_preserve_i64_and_reject_bad_data() {
    let echo = echo_array();
    assert_eq!(
        output(
            &echo,
            "\t3\x0b-9223372036854775808\x0c9223372036854775807\r-0\n"
        )
        .unwrap(),
        "[-9223372036854775808,9223372036854775807,0]\n"
    );
    assert_eq!(output(&echo, "0").unwrap(), "[]\n");
    for input in [
        "",
        "-1",
        "10001",
        "2 4",
        "1 +3",
        "1 -",
        "1 3x",
        "1 9223372036854775808",
        "1 -9223372036854775809",
        "1 3 7",
        "1 3\0",
        "1\u{a0}3",
    ] {
        assert!(output(&echo, input).is_err(), "input {input:?}");
    }
    assert!(
        output(&echo, &" ".repeat(262145))
            .unwrap_err()
            .contains("262144")
    );
    let indexing = program(
        &[5, 2, 2, 5],
        &[
            op(0x31, 5, &[0]),
            op(0x30, 2, &[1]),
            op(0x33, 2, &[2, 0, 1]),
            array(3, &[2]),
            op(0x52, 0, &[3]),
        ],
    );
    assert_eq!(output(&indexing, "2 7 8 1").unwrap(), "[8]\n");
    for input in ["2 7 8 -1", "2 7 8 2", "0 0"] {
        assert!(output(&indexing, input).unwrap_err().contains("bounds"));
    }
}

fn generic_map_program() -> Vec<u8> {
    // Overwrite an existing map key, use negative/maximum i64 keys and a fallback.
    program(
        &[6, 2, 2, 2, 2, 2, 2, 5, 3],
        &[
            op(0x40, 6, &[0]),
            int(1, i64::MIN),
            int(2, 4),
            int(3, -1),
            op(0x42, 0, &[0, 1, 2]),
            int(2, 17),
            op(0x42, 0, &[0, 1, 2]),
            op(0x41, 2, &[4, 0, 1, 3]),
            int(1, i64::MAX),
            op(0x41, 2, &[5, 0, 1, 3]),
            op(0x13, 2, &[6, 4, 5]),
            boolean(8, 1),
            op(0x51, 0, &[8, 13, 13]),
            array(7, &[4, 5, 6]),
            op(0x52, 0, &[7]),
        ],
    )
}

#[test]
fn generic_map_overwrite_missing_key_and_arithmetic_are_reference_operations() {
    let source = generic_map_program();
    assert_eq!(output(&source, "").unwrap(), "[17,-1,18]\n");
    let compiled = language::compile_binary(&source).unwrap();
    assert_eq!(compiled.encode_binary(), source);
    let llvm = compiled.emit_llvm();
    for name in [
        "@rv2_map_new",
        "@rv2_map_get",
        "@rv2_map_put",
        "@llvm.ssub.with.overflow.i64",
        "@rv2_array_make",
    ] {
        assert!(llvm.contains(name));
    }
    assert!(!llvm.contains("two_sum"));
}

#[test]
fn arithmetic_fuel_and_allocation_failures_are_runtime_checks() {
    for (opcode, first) in [(0x11, i64::MAX), (0x13, i64::MIN)] {
        let source = program(
            &[2, 2, 5],
            &[
                int(0, first),
                int(1, 1),
                op(opcode, 2, &[0, 0, 1]),
                array(2, &[0]),
                op(0x52, 0, &[2]),
            ],
        );
        assert!(language::inspect_binary(&source).is_ok());
        assert!(output(&source, "").unwrap_err().contains("overflow"));
    }
    let forever = program(&[2], &[op(0x50, 0, &[0])]);
    assert!(language::inspect_binary(&forever).is_ok());
    assert!(output(&forever, "").unwrap_err().contains("1000000"));
    let allocations = program(&[6], &[op(0x40, 6, &[0]), op(0x50, 0, &[0])]);
    assert!(
        output(&allocations, "")
            .unwrap_err()
            .contains("allocation budget")
    );
}

#[test]
fn mutated_sources_never_panic_and_accepted_bytes_roundtrip() {
    let original = include_bytes!("../examples/two-sum.rvb");
    let mut seed = 0x7269766574u64;
    for _ in 0..8000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut source = original.to_vec();
        source[(seed as usize) % original.len()] = (seed >> 32) as u8;
        if let Ok(compiled) = language::compile_binary(&source) {
            assert_eq!(compiled.encode_binary(), source);
            let _ = compiled.emit_llvm();
            assert!(language::inspect_binary(&source).is_ok());
        }
    }
}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        Self(native::unique_directory(&std::env::temp_dir(), "rivet-compute-tests").unwrap())
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn generic_native_programs_match_reference_under_both_profiles() {
    let scratch = Scratch::new();
    let cases = [
        (generic_map_program(), ""),
        (
            program(
                &[2, 2, 2, 3, 5],
                &[
                    int(0, 0),
                    int(1, 1),
                    int(2, 10000),
                    array(4, &[]),
                    op(0x15, 3, &[3, 0, 2]),
                    op(0x51, 0, &[3, 6, 9]),
                    array(4, &[0]),
                    op(0x11, 2, &[0, 0, 1]),
                    op(0x50, 0, &[4]),
                    op(0x52, 0, &[4]),
                ],
            ),
            "",
        ),
        (echo_array(), "3 -9223372036854775808 9223372036854775807 0"),
        (
            program(
                &[2, 2, 2, 3, 5],
                &[
                    int(0, 0),
                    int(1, 1),
                    int(2, 3),
                    op(0x15, 3, &[3, 0, 2]),
                    op(0x51, 0, &[3, 5, 7]),
                    op(0x11, 2, &[0, 0, 1]),
                    op(0x50, 0, &[3]),
                    array(4, &[0]),
                    op(0x52, 0, &[4]),
                ],
            ),
            "",
        ),
        (
            program(
                &[5, 2],
                &[array(0, &[]), op(0x52, 0, &[0]), op(0x11, 2, &[1, 1, 1])],
            ),
            "",
        ),
        (
            program(
                &[2, 2, 5],
                &[
                    int(0, i64::MAX),
                    int(1, 1),
                    op(0x11, 2, &[0, 0, 1]),
                    array(2, &[0]),
                    op(0x52, 0, &[2]),
                ],
            ),
            "",
        ),
        (
            program(
                &[2, 2, 5],
                &[
                    int(0, i64::MIN),
                    int(1, 1),
                    op(0x13, 2, &[0, 0, 1]),
                    array(2, &[0]),
                    op(0x52, 0, &[2]),
                ],
            ),
            "",
        ),
        (program(&[2], &[op(0x50, 0, &[0])]), ""),
        (program(&[6], &[op(0x40, 6, &[0]), op(0x50, 0, &[0])]), ""),
    ];
    for profile in [BuildProfile::Baseline, BuildProfile::Size] {
        for (index, (source, input)) in cases.iter().enumerate() {
            let compiled = language::compile_binary(source).unwrap();
            let artifact = native::build_native(
                &compiled,
                &scratch.0.join(format!("{}-{index}", profile.name())),
                profile,
            )
            .unwrap();
            let reference = output(source, input);
            let native = run_console_with_input(&artifact, input);
            match reference {
                Ok(expected) => assert_eq!(
                    native.unwrap().stdout,
                    expected,
                    "case {index} {}",
                    profile.name()
                ),
                Err(_) => assert!(native.is_err(), "case {index} {}", profile.name()),
            }
        }
    }
}
