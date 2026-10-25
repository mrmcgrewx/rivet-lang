//! Generic RVB2 list/copy/division semantics authored directly as numeric bytes.
use rivet::{
    language::{self, Execution},
    native::{self, BuildProfile},
    native_process::run_console_with_input,
};
use std::{fs, path::PathBuf};

fn refs(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}
fn record(code: u8, tag: u8, payload: &[u8]) -> Vec<u8> {
    let mut result = vec![code, tag];
    result.extend((payload.len() as u16).to_le_bytes());
    result.extend(payload);
    result
}
fn op(code: u8, tag: u8, values: &[u16]) -> Vec<u8> {
    record(code, tag, &refs(values))
}
fn int(destination: u16, value: i64) -> Vec<u8> {
    let mut payload = refs(&[destination]);
    payload.extend(value.to_le_bytes());
    record(2, 2, &payload)
}
fn boolean(destination: u16, value: bool) -> Vec<u8> {
    let mut payload = refs(&[destination]);
    payload.push(value.into());
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
    let length = (result.len() - 16) as u32;
    result[12..16].copy_from_slice(&length.to_le_bytes());
    result
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
        _ => panic!("compute output expected"),
    }
}
fn echo_list() -> Vec<u8> {
    program(&[7], &[op(0x65, 7, &[0]), op(0x53, 0, &[0])])
}
fn arithmetic(operation: u8) -> Vec<u8> {
    program(
        &[2, 2, 5],
        &[
            op(0x30, 2, &[0]),
            op(0x30, 2, &[1]),
            op(operation, 2, &[0, 0, 1]),
            array(2, &[0]),
            op(0x52, 0, &[2]),
        ],
    )
}
fn copied_values() -> Vec<u8> {
    program(
        &[2, 2, 3, 3, 5, 5, 6, 6, 7, 7, 2, 2, 3, 5],
        &[
            int(0, 42),
            op(0x16, 2, &[1, 0]),
            boolean(2, true),
            op(0x16, 3, &[3, 2]),
            op(0x54, 0, &[3]),
            array(4, &[0]),
            op(0x16, 5, &[5, 4]),
            op(0x40, 6, &[6]),
            op(0x16, 6, &[7, 6]),
            int(10, 9),
            op(0x42, 0, &[7, 10, 1]),
            int(11, -1),
            op(0x41, 2, &[1, 6, 10, 11]),
            op(0x60, 7, &[8]),
            op(0x61, 7, &[8, 1, 8]),
            op(0x16, 7, &[9, 8]),
            op(0x60, 7, &[8]),
            op(0x63, 2, &[1, 9]),
            int(10, 0),
            op(0x33, 2, &[0, 5, 10]),
            op(0x14, 3, &[12, 0, 1]),
            op(0x54, 0, &[12]),
            array(13, &[0, 1]),
            op(0x52, 0, &[13]),
        ],
    )
}
fn shared_tails() -> Vec<u8> {
    program(
        &[7, 2, 7, 7, 7, 7, 2, 2, 2, 2, 3, 5],
        &[
            op(0x60, 7, &[0]),
            int(1, 7),
            op(0x61, 7, &[2, 1, 0]),
            op(0x16, 7, &[3, 2]),
            int(1, 9),
            op(0x61, 7, &[4, 1, 2]),
            op(0x64, 7, &[5, 4]),
            op(0x63, 2, &[6, 3]),
            op(0x63, 2, &[7, 5]),
            op(0x66, 2, &[8, 4]),
            op(0x66, 2, &[9, 2]),
            op(0x64, 7, &[3, 3]),
            op(0x62, 3, &[10, 3]),
            op(0x54, 0, &[10]),
            array(11, &[6, 7, 8, 9]),
            op(0x52, 0, &[11]),
        ],
    )
}
fn overfull_cons() -> Vec<u8> {
    program(
        &[7, 2],
        &[
            op(0x65, 7, &[0]),
            int(1, 5),
            op(0x61, 7, &[0, 1, 0]),
            op(0x53, 0, &[0]),
        ],
    )
}
fn quota_before_missing_input() -> Vec<u8> {
    let mut operations = vec![op(0x40, 6, &[0]); 85];
    operations.extend([op(0x65, 7, &[1]), op(0x65, 7, &[1]), op(0x53, 0, &[1])]);
    program(&[6, 7], &operations)
}

#[test]
fn all_new_operations_roundtrip_and_decode_without_execution() {
    let source = include_bytes!("../examples/add-two-numbers.rvb");
    let compiled = language::compile_binary(source).unwrap();
    assert_eq!(compiled.encode_binary(), source);
    let inspection = language::inspect_binary(source).unwrap();
    assert!(inspection.ir.contains("IntList"));
    for operation in [
        "list_nil",
        "list_cons",
        "list_is_nil",
        "list_value",
        "list_next",
        "read_list",
        "list_len",
        "return_list",
        "assert",
        "copy",
        "div",
        "rem",
    ] {
        assert!(inspection.ir.contains(operation), "{operation}");
    }
    assert!(compiled.execute().is_err());
    assert_eq!(output(source, "3 2 4 3 3 5 6 4").unwrap(), "[7,0,8]\n");
    for source in [
        echo_list(),
        copied_values(),
        shared_tails(),
        arithmetic(0x17),
        arithmetic(0x18),
    ] {
        assert_eq!(
            language::compile_binary(&source).unwrap().encode_binary(),
            source
        );
    }
}

#[test]
fn copies_preserve_scalar_values_immutable_handles_and_mutable_map_aliases() {
    assert_eq!(output(&copied_values(), "").unwrap(), "[42,42]\n");
    assert_eq!(output(&shared_tails(), "").unwrap(), "[7,7,2,1]\n");
    // Copying a nil handle initializes a destination, and both remain nil.
    let source = program(
        &[7, 7, 3],
        &[
            op(0x60, 7, &[0]),
            op(0x16, 7, &[1, 0]),
            op(0x62, 3, &[2, 1]),
            op(0x54, 0, &[2]),
            op(0x53, 0, &[0]),
        ],
    );
    assert_eq!(output(&source, "").unwrap(), "[]\n");
}

#[test]
fn signed_division_and_remainder_truncate_toward_zero_and_check_both_undefined_cases() {
    let div = arithmetic(0x17);
    let rem = arithmetic(0x18);
    for (a, b, quotient, remainder) in [
        (-7, 3, -2, -1),
        (7, -3, -2, 1),
        (-7, -3, 2, -1),
        (0, -3, 0, 0),
        (i64::MIN, 1, i64::MIN, 0),
        (i64::MIN, 3, -3074457345618258602, -2),
        (i64::MAX, -1, -i64::MAX, 0),
        (i64::MAX, i64::MIN, 0, i64::MAX),
    ] {
        let input = format!("{a} {b}");
        assert_eq!(output(&div, &input).unwrap(), format!("[{quotient}]\n"));
        assert_eq!(output(&rem, &input).unwrap(), format!("[{remainder}]\n"));
    }
    for (source, arithmetic_instruction) in [(&div, " = sdiv"), (&rem, " = srem")] {
        assert!(output(source, "1 0").unwrap_err().contains("zero"));
        assert!(
            output(source, "-9223372036854775808 -1")
                .unwrap_err()
                .contains("overflow")
        );
        let llvm = language::compile_binary(source).unwrap().emit_llvm();
        let arithmetic = llvm.find(arithmetic_instruction).unwrap();
        assert!(llvm[..arithmetic].contains("label %division_zero"));
        assert!(llvm[..arithmetic].contains("icmp eq i64"));
        assert!(llvm[..arithmetic].contains("-9223372036854775808"));
        assert!(llvm[..arithmetic].contains("label %overflow"));
        assert!(!llvm.contains(" select "));
    }
}

#[test]
fn nil_is_initialized_but_dereference_fails_and_assertions_preserve_order() {
    for (operation, tag, destination) in [(0x63, 2, 1), (0x64, 7, 0)] {
        let source = program(
            &[7, 2],
            &[
                op(0x60, 7, &[0]),
                op(operation, tag, &[destination, 0]),
                op(0x53, 0, &[0]),
            ],
        );
        assert!(language::inspect_binary(&source).is_ok());
        assert!(output(&source, "").unwrap_err().contains("nil"));
    }
    let source = program(
        &[7, 3, 2],
        &[
            op(0x60, 7, &[0]),
            boolean(1, false),
            op(0x54, 0, &[1]),
            op(0x63, 2, &[2, 0]),
            op(0x53, 0, &[0]),
        ],
    );
    let error = output(&source, "").unwrap_err();
    assert!(error.contains("instruction @2: assertion failed"));
    assert!(!error.contains("nil"));
    let llvm = language::compile_binary(&source).unwrap().emit_llvm();
    assert!(llvm.contains("assertion failed at instruction @2"));
    let length_of_nil = program(
        &[7, 2, 5],
        &[
            op(0x60, 7, &[0]),
            op(0x66, 2, &[1, 0]),
            array(2, &[1]),
            op(0x52, 0, &[2]),
        ],
    );
    assert_eq!(output(&length_of_nil, "").unwrap(), "[0]\n");
}

#[test]
fn list_io_is_generic_exact_bounded_and_uses_nonrecursive_lifetimes() {
    let echo = echo_list();
    assert_eq!(output(&echo, "0").unwrap(), "[]\n");
    assert_eq!(
        output(&echo, "3 -9223372036854775808 42 9223372036854775807").unwrap(),
        "[-9223372036854775808,42,9223372036854775807]\n"
    );
    for input in [
        "",
        "-1",
        "10001",
        "2 5",
        "1 +4",
        "1 -9223372036854775809",
        "0 7",
        "1 8x",
    ] {
        assert!(output(&echo, input).is_err(), "{input}");
    }
    let input = format!("10000 {}", vec![i64::MIN.to_string(); 10000].join(" "));
    let expected = format!("[{}]\n", vec![i64::MIN.to_string(); 10000].join(","));
    let compiled = language::compile_binary(&echo).unwrap();
    // A small thread stack makes accidental recursive node destruction observable.
    let result = std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(move || compiled.execute_with_input(&input))
        .unwrap()
        .join()
        .unwrap()
        .unwrap();
    match result {
        Execution::Console { stdout } => assert_eq!(stdout, expected),
        _ => unreachable!(),
    }
    let ten_thousand = format!("10000 {}", vec!["0"; 10000].join(" "));
    assert!(
        output(&overfull_cons(), &ten_thousand)
            .unwrap_err()
            .contains("10000")
    );
    let quota_input = format!("{ten_thousand} 1000");
    let error = output(&quota_before_missing_input(), &quota_input).unwrap_err();
    assert!(error.contains("allocation budget"), "{error}");
    assert!(!error.contains("missing"));
}

#[test]
fn new_record_payloads_tags_and_typed_references_are_checked() {
    let types = [7, 2, 3, 5, 6];
    for (bad, reason) in [
        (op(0x16, 0, &[0, 0]), "copy requires"),
        (op(0x16, 4, &[0, 0]), "copy requires"),
        (op(0x16, 7, &[0, 1]), "expected IntList"),
        (op(0x16, 2, &[1, 8]), "outside the declared"),
        (op(0x17, 3, &[1, 1, 1]), "result type tag"),
        (op(0x18, 2, &[1, 0, 1]), "expected Int"),
        (op(0x53, 0, &[3]), "expected IntList"),
        (op(0x54, 0, &[1]), "expected Bool"),
        (op(0x60, 7, &[1]), "expected IntList"),
        (op(0x61, 7, &[0, 2, 0]), "expected Int"),
        (op(0x62, 3, &[2, 3]), "expected IntList"),
        (op(0x63, 2, &[1, 4]), "expected IntList"),
        (op(0x64, 7, &[0, 4]), "expected IntList"),
        (op(0x65, 5, &[0]), "result type tag"),
        (op(0x66, 2, &[1, 8]), "outside the declared"),
        (op(0x16, 7, &[0]), "truncated"),
        (op(0x60, 7, &[0, 0]), "trailing bytes"),
        (op(0x61, 7, &[0, 1]), "truncated"),
        (op(0x53, 0, &[]), "truncated"),
    ] {
        reject(&program(&types, &[bad, op(0x50, 0, &[0])]), reason);
    }
}

#[test]
fn definite_assignment_accounts_for_copy_lists_assertions_and_return_terminators() {
    let types = [7, 2, 3];
    for bad in [
        op(0x16, 7, &[0, 0]),
        op(0x17, 2, &[1, 1, 1]),
        op(0x18, 2, &[1, 1, 1]),
        op(0x53, 0, &[0]),
        op(0x54, 0, &[2]),
        op(0x61, 7, &[0, 1, 0]),
        op(0x62, 3, &[2, 0]),
        op(0x63, 2, &[1, 0]),
        op(0x64, 7, &[0, 0]),
        op(0x66, 2, &[1, 0]),
    ] {
        reject(
            &program(&types, &[bad, op(0x50, 0, &[0])]),
            "definitely initialized",
        );
    }
    reject(
        &program(
            &types,
            &[
                boolean(2, true),
                op(0x51, 0, &[2, 2, 3]),
                op(0x60, 7, &[0]),
                op(0x53, 0, &[0]),
            ],
        ),
        "definitely initialized",
    );
    reject(
        &program(&[3], &[boolean(0, false), op(0x54, 0, &[0])]),
        "falls through",
    );
    // Return-list has no fallthrough; unreachable malformed reads are tolerated,
    // but the record itself remains type checked by the previous test.
    let source = program(
        &types,
        &[op(0x60, 7, &[0]), op(0x53, 0, &[0]), op(0x63, 2, &[1, 0])],
    );
    assert_eq!(output(&source, "").unwrap(), "[]\n");
}

#[test]
fn new_opcode_mutations_are_bounded_and_accepted_sources_roundtrip() {
    let source = include_bytes!("../examples/add-two-numbers.rvb");
    for length in 0..source.len() {
        assert!(language::compile_binary(&source[..length]).is_err());
    }
    let mut seed = 0x1a57_c0de_u64;
    for _ in 0..4000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut mutated = source.to_vec();
        mutated[seed as usize % source.len()] = (seed >> 32) as u8;
        if let Ok(compiled) = language::compile_binary(&mutated) {
            assert_eq!(compiled.encode_binary(), mutated);
            assert!(language::inspect_binary(&mutated).is_ok());
            let _ = compiled.emit_llvm();
        }
    }
}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        Self(native::unique_directory(&std::env::temp_dir(), "rivet-list-tests").unwrap())
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn generic_lists_aliases_division_and_failures_match_native_both_profiles() {
    let scratch = Scratch::new();
    let deep_input = format!("10000 {}", vec![i64::MIN.to_string(); 10000].join(" "));
    let quota_input = format!("10000 {} 1000", vec!["0"; 10000].join(" "));
    let cases = [
        (copied_values(), vec!["".into()]),
        (shared_tails(), vec!["".into()]),
        (
            echo_list(),
            vec![
                "0".into(),
                "3 -9223372036854775808 42 9223372036854775807".into(),
                deep_input.clone(),
                "0 1".into(),
                "10001".into(),
                "2 4".into(),
            ],
        ),
        (
            arithmetic(0x17),
            vec![
                "-7 3".into(),
                "7 -3".into(),
                "-7 -3".into(),
                "-9223372036854775808 1".into(),
                "-9223372036854775808 3".into(),
                "9223372036854775807 -9223372036854775808".into(),
                "1 0".into(),
                "-9223372036854775808 -1".into(),
            ],
        ),
        (
            arithmetic(0x18),
            vec![
                "-7 3".into(),
                "7 -3".into(),
                "-7 -3".into(),
                "-9223372036854775808 1".into(),
                "-9223372036854775808 3".into(),
                "9223372036854775807 -9223372036854775808".into(),
                "1 0".into(),
                "-9223372036854775808 -1".into(),
            ],
        ),
        (overfull_cons(), vec![deep_input]),
        (quota_before_missing_input(), vec![quota_input]),
        (
            program(
                &[7, 2],
                &[op(0x60, 7, &[0]), op(0x63, 2, &[1, 0]), op(0x53, 0, &[0])],
            ),
            vec!["".into()],
        ),
        (
            program(
                &[7],
                &[op(0x60, 7, &[0]), op(0x64, 7, &[0, 0]), op(0x53, 0, &[0])],
            ),
            vec!["".into()],
        ),
        (
            program(
                &[7, 3],
                &[
                    op(0x60, 7, &[0]),
                    boolean(1, false),
                    op(0x54, 0, &[1]),
                    op(0x53, 0, &[0]),
                ],
            ),
            vec!["".into()],
        ),
    ];
    for profile in [BuildProfile::Baseline, BuildProfile::Size] {
        for (index, (source, inputs)) in cases.iter().enumerate() {
            let compiled = language::compile_binary(source).unwrap();
            let artifact = native::build_native(
                &compiled,
                &scratch.0.join(format!("{}-{index}", profile.name())),
                profile,
            )
            .unwrap();
            for input in inputs {
                match output(source, input) {
                    Ok(expected) => assert_eq!(
                        run_console_with_input(&artifact, input).unwrap().stdout,
                        expected,
                        "case{index} {}",
                        profile.name()
                    ),
                    Err(reference) => {
                        let error = run_console_with_input(&artifact, input).unwrap_err();
                        assert_eq!(error.exit_code, Some(1));
                        let diagnostic = format!("{} {}", error.message, error.stderr);
                        for keyword in [
                            "zero",
                            "overflow",
                            "nil",
                            "allocation",
                            "assertion failed",
                            "unconsumed",
                            "missing",
                            "10000",
                        ] {
                            if reference.contains(keyword) {
                                assert!(
                                    diagnostic.contains(keyword),
                                    "case{index}: {reference:?} vs{diagnostic:?}"
                                );
                            }
                        }
                        if index == 9 {
                            assert!(diagnostic.contains("instruction @2"));
                        }
                    }
                }
            }
        }
    }
}
