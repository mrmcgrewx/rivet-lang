//! Generic binary64 source, precision and checked-native semantics; no median solver.
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
fn float(destination: u16, bits: u64) -> Vec<u8> {
    let mut payload = refs(&[destination]);
    payload.extend(bits.to_le_bytes());
    record(4, 8, &payload)
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
fn literal(bits: u64) -> Vec<u8> {
    program(
        &[8, 8],
        &[float(0, bits), op(0x16, 8, &[1, 0]), op(0x55, 0, &[1])],
    )
}
fn conversion() -> Vec<u8> {
    program(
        &[2, 8],
        &[op(0x30, 2, &[0]), op(0x19, 8, &[1, 0]), op(0x55, 0, &[1])],
    )
}
fn literal_bits() -> Vec<u64> {
    let mut values = vec![
        0,
        1u64 << 63,
        1,
        (1u64 << 63) | 1,
        0x000fffffffffffff,
        f64::MIN_POSITIVE.to_bits(),
        f64::MAX.to_bits(),
        f64::MIN.to_bits(),
        1f64.to_bits(),
        (-1f64).to_bits(),
        2.5f64.to_bits(),
        (1f64 / 3.).to_bits(),
        std::f64::consts::PI.to_bits(),
        0x4340000000000000,
        0x4340000000000001,
        0x43e0000000000000,
        0xc3e0000000000000,
        1e100f64.to_bits(),
        1e-100f64.to_bits(),
        (-1e-100f64).to_bits(),
    ];
    let mut seed = 0x9184_7af2_c015_7d32u64;
    while values.len() < 40 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        if f64::from_bits(seed).is_finite() {
            values.push(seed);
        }
    }
    values
}
fn float_selector(values: &[u64]) -> Vec<u8> {
    // One native program exercises many literal bit patterns without rebuilding.
    let mut operations = vec![op(0x30, 2, &[0])];
    for (index, bits) in values.iter().enumerate() {
        let pc = operations.len() as u16;
        operations.extend([
            int(1, index as i64),
            op(0x14, 3, &[2, 0, 1]),
            op(0x51, 0, &[2, pc + 3, pc + 6]),
            float(3, *bits),
            op(0x16, 8, &[4, 3]),
            op(0x55, 0, &[4]),
        ]);
    }
    let fallback = operations.len() as u16;
    operations.extend([
        boolean(2, false),
        op(0x54, 0, &[2]),
        op(0x50, 0, &[fallback]),
    ]);
    program(&[2, 2, 3, 8, 8], &operations)
}
#[derive(Clone, Copy)]
struct ArithmeticCase {
    left: u64,
    right: u64,
    result: Result<u64, &'static str>,
}
fn case(left: f64, right: f64, result: Result<f64, &'static str>) -> ArithmeticCase {
    ArithmeticCase {
        left: left.to_bits(),
        right: right.to_bits(),
        result: result.map(f64::to_bits),
    }
}
fn add_cases() -> Vec<ArithmeticCase> {
    vec![
        case(1.25, 2.5, Ok(3.75)),
        case(-0., -0., Ok(-0.)),
        case(-0., 0., Ok(0.)),
        case(f64::from_bits(1), f64::from_bits(1), Ok(f64::from_bits(2))),
        case(f64::MAX, f64::MAX, Err("nonfinite")),
        case(-f64::MAX, -f64::MAX, Err("nonfinite")),
        case(1., -1., Ok(0.)),
        case(9007199254740992., 1., Ok(9007199254740992.)),
        case(9007199254740994., 1., Ok(9007199254740996.)),
        case(f64::MAX, -f64::MAX, Ok(0.)),
        case(
            f64::MIN_POSITIVE,
            -f64::from_bits(0x000fffffffffffff),
            Ok(f64::from_bits(1)),
        ),
    ]
}
fn div_cases() -> Vec<ArithmeticCase> {
    vec![
        case(5., 2., Ok(2.5)),
        case(-5., 2., Ok(-2.5)),
        case(1., 3., Ok(f64::from_bits(0x3fd5555555555555))),
        case(-0., 2., Ok(-0.)),
        case(0., -2., Ok(-0.)),
        case(f64::from_bits(1), 2., Ok(0.)),
        case(-f64::from_bits(1), 2., Ok(-0.)),
        case(f64::from_bits(3), 2., Ok(f64::from_bits(2))),
        case(
            f64::MIN_POSITIVE,
            2.,
            Ok(f64::from_bits(0x0008000000000000)),
        ),
        case(f64::MAX, f64::MIN_POSITIVE, Err("nonfinite")),
        case(-f64::MAX, f64::MIN_POSITIVE, Err("nonfinite")),
        case(f64::MAX, 1., Ok(f64::MAX)),
        case(f64::from_bits(1), f64::from_bits(1), Ok(1.)),
        case(2., 0., Err("zero")),
        case(2., -0., Err("zero")),
        case(0., 0., Err("zero")),
    ]
}
fn arithmetic_selector(opcode: u8, cases: &[ArithmeticCase]) -> Vec<u8> {
    let common = (1 + 6 * cases.len() + 3) as u16;
    let mut operations = vec![op(0x30, 2, &[0])];
    for (index, case) in cases.iter().enumerate() {
        let pc = operations.len() as u16;
        operations.extend([
            int(1, index as i64),
            op(0x14, 3, &[2, 0, 1]),
            op(0x51, 0, &[2, pc + 3, pc + 6]),
            float(3, case.left),
            float(4, case.right),
            op(0x50, 0, &[common]),
        ]);
    }
    let fallback = operations.len() as u16;
    operations.extend([
        boolean(2, false),
        op(0x54, 0, &[2]),
        op(0x50, 0, &[fallback]),
        op(opcode, 8, &[3, 3, 4]),
        op(0x55, 0, &[3]),
    ]);
    program(&[2, 2, 3, 8, 8], &operations)
}
fn assert_scientific(text: &str, expected_bits: u64) {
    assert!(text.ends_with('\n'));
    let number = text.trim_end_matches('\n');
    assert_eq!(number.parse::<f64>().unwrap().to_bits(), expected_bits);
    let (mantissa, exponent) = number.split_once('e').unwrap();
    let mantissa = mantissa.strip_prefix('-').unwrap_or(mantissa);
    let (leading, fraction) = mantissa.split_once('.').unwrap();
    assert_eq!(leading.len(), 1);
    assert!(leading.bytes().all(|v| v.is_ascii_digit()));
    assert_eq!(fraction.len(), 16);
    assert!(fraction.bytes().all(|v| v.is_ascii_digit()));
    assert!(exponent.starts_with(['+', '-']));
    assert!(exponent.len() >= 3);
    assert!(exponent[1..].bytes().all(|v| v.is_ascii_digit()));
}

#[test]
fn literals_copies_and_scientific_output_preserve_exact_finite_bits() {
    for bits in literal_bits() {
        let source = literal(bits);
        let compiled = language::compile_binary(&source).unwrap();
        assert_eq!(compiled.encode_binary(), source);
        assert_scientific(&output(&source, "").unwrap(), bits);
        let inspection = language::inspect_binary(&source).unwrap();
        assert!(inspection.ir.contains(&format!("bits 0x{bits:016x}")));
        assert!(
            inspection
                .instructions
                .iter()
                .all(|item| item.literal.is_none())
        );
        assert!(
            compiled
                .emit_llvm()
                .contains(&format!("double 0x{bits:016X}"))
        );
    }
    for (value, expected) in [
        (0., "0.0000000000000000e+00\n"),
        (-0., "-0.0000000000000000e+00\n"),
        (2., "2.0000000000000000e+00\n"),
        (2.5, "2.5000000000000000e+00\n"),
        (f64::MAX, "1.7976931348623157e+308\n"),
        (f64::from_bits(1), "4.9406564584124654e-324\n"),
    ] {
        assert_eq!(output(&literal(value.to_bits()), "").unwrap(), expected);
    }
}

#[test]
fn nonfinite_literals_and_malformed_float_records_are_rejected_before_execution() {
    for bits in [
        0x7ff0000000000000,
        0xfff0000000000000,
        0x7ff0000000000001,
        0x7ff8000000000000,
        0xfff8000000000001,
        u64::MAX,
    ] {
        reject(&literal(bits), "must be finite");
        // Even unreachable declarations must contain legal finite literal bits.
        reject(
            &program(&[8], &[float(0, 0), op(0x55, 0, &[0]), float(0, bits)]),
            "must be finite",
        );
    }
    let types = [2, 8, 3];
    for (bad, reason) in [
        (record(4, 2, &[0; 10]), "result type tag"),
        (float(0, 0), "expected Float"),
        (
            record(4, 8, &[1, 0, 0, 0, 0, 0, 0, 0, 0]),
            "truncated Float literal",
        ),
        (
            record(4, 8, &[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            "trailing bytes",
        ),
        (op(0x19, 8, &[1, 1]), "expected Int"),
        (op(0x1a, 8, &[1, 0, 1]), "expected Float"),
        (op(0x1b, 8, &[1, 1, 3]), "outside the declared"),
        (op(0x55, 0, &[0]), "expected Float"),
        (op(0x16, 8, &[1, 0]), "expected Float"),
        (op(0x55, 8, &[1]), "result type tag"),
    ] {
        reject(&program(&types, &[bad, op(0x50, 0, &[0])]), reason);
    }
}

#[test]
fn int_conversion_rounds_ties_to_even_at_binary64_precision_boundaries() {
    for (integer, bits) in conversion_cases() {
        assert_scientific(&output(&conversion(), &integer.to_string()).unwrap(), bits);
    }
}
fn conversion_cases() -> Vec<(i64, u64)> {
    vec![
        (0, 0),
        (1, 0x3ff0000000000000),
        (-1, 0xbff0000000000000),
        (9007199254740991, 0x433fffffffffffff),
        (9007199254740992, 0x4340000000000000),
        (9007199254740993, 0x4340000000000000),
        (9007199254740994, 0x4340000000000001),
        (9007199254740995, 0x4340000000000002),
        (-9007199254740993, 0xc340000000000000),
        (-9007199254740995, 0xc340000000000002),
        (i64::MAX, 0x43e0000000000000),
        (i64::MIN, 0xc3e0000000000000),
    ]
}

#[test]
fn arithmetic_has_ieee_rounding_signed_zero_subnormals_and_checked_failures() {
    for (opcode, cases) in [(0x1a, add_cases()), (0x1b, div_cases())] {
        let source = arithmetic_selector(opcode, &cases);
        for (index, case) in cases.iter().enumerate() {
            match case.result {
                Ok(bits) => assert_scientific(&output(&source, &index.to_string()).unwrap(), bits),
                Err(reason) => assert!(
                    output(&source, &index.to_string())
                        .unwrap_err()
                        .contains(reason)
                ),
            }
        }
        let llvm = language::compile_binary(&source).unwrap().emit_llvm();
        assert!(llvm.contains("9218868437227405312"));
        assert!(llvm.contains("label %float_nonfinite"));
        assert!(!llvm.contains(" fast "));
        assert!(!llvm.contains(" nnan "));
        assert!(!llvm.contains(" ninf "));
        if opcode == 0x1b {
            assert!(
                llvm.find("label %float_division_zero").unwrap() < llvm.find(" = fdiv").unwrap()
            );
        }
    }
}

fn unused_invalid(opcode: u8) -> Vec<u8> {
    let (a, b) = if opcode == 0x1a {
        (f64::MAX, f64::MAX)
    } else {
        (1., -0.)
    };
    program(
        &[8, 8, 8, 8],
        &[
            float(0, a.to_bits()),
            float(1, b.to_bits()),
            op(opcode, 8, &[2, 0, 1]),
            float(3, 2f64.to_bits()),
            op(0x55, 0, &[3]),
        ],
    )
}
#[test]
fn float_failures_remain_ordered_when_results_are_unused_and_return_checks_input() {
    assert!(
        output(&unused_invalid(0x1a), "")
            .unwrap_err()
            .contains("instruction @2: nonfinite")
    );
    assert!(
        output(&unused_invalid(0x1b), "")
            .unwrap_err()
            .contains("instruction @2: floating-point division by zero")
    );
    assert!(output(&literal(0), "7").unwrap_err().contains("unconsumed"));
    assert_eq!(
        output(&literal(0), " \t\r\n").unwrap(),
        "0.0000000000000000e+00\n"
    );
}

#[test]
fn float_initialization_types_loops_and_return_terminators_are_validated() {
    let types = [8, 2, 3];
    for bad in [
        op(0x16, 8, &[0, 0]),
        op(0x19, 8, &[0, 1]),
        op(0x1a, 8, &[0, 0, 0]),
        op(0x1b, 8, &[0, 0, 0]),
        op(0x55, 0, &[0]),
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
                float(0, 0),
                op(0x55, 0, &[0]),
            ],
        ),
        "definitely initialized",
    );
    reject(&program(&[8], &[float(0, 0)]), "falls through");
    let unreachable = program(
        &[8, 8],
        &[float(0, 0), op(0x55, 0, &[0]), op(0x1a, 8, &[1, 1, 1])],
    );
    assert_eq!(
        output(&unreachable, "").unwrap(),
        "0.0000000000000000e+00\n"
    );
    assert_eq!(
        output(&float_loop(), "").unwrap(),
        "1.5000000000000000e+00\n"
    );
}
fn float_loop() -> Vec<u8> {
    program(
        &[2, 2, 2, 3, 8, 8],
        &[
            int(0, 0),
            int(1, 1),
            int(2, 3),
            float(4, 0),
            float(5, 0.5f64.to_bits()),
            op(0x15, 3, &[3, 0, 2]),
            op(0x51, 0, &[3, 7, 10]),
            op(0x1a, 8, &[4, 4, 5]),
            op(0x11, 2, &[0, 0, 1]),
            op(0x50, 0, &[5]),
            op(0x55, 0, &[4]),
        ],
    )
}

#[test]
fn finite_bit_mutations_roundtrip_and_malformed_records_never_panic() {
    let original = float_selector(&literal_bits());
    for length in 0..original.len() {
        assert!(language::compile_binary(&original[..length]).is_err());
    }
    let mut seed = 0xf10a_7a88_5193_0201u64;
    for _ in 0..4000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        if f64::from_bits(seed).is_finite() {
            let source = literal(seed);
            let compiled = language::compile_binary(&source).unwrap();
            assert_eq!(compiled.encode_binary(), source);
            assert_scientific(&output(&source, "").unwrap(), seed);
        }
        let mut source = original.clone();
        source[seed as usize % original.len()] = (seed >> 32) as u8;
        if let Ok(compiled) = language::compile_binary(&source) {
            assert_eq!(compiled.encode_binary(), source);
            assert!(language::inspect_binary(&source).is_ok());
            let _ = compiled.emit_llvm();
        }
    }
}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        Self(native::unique_directory(&std::env::temp_dir(), "rivet-float-tests").unwrap())
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn generic_float_programs_match_native_baseline_size_and_speed_profiles() {
    let scratch = Scratch::new();
    let literals = literal_bits();
    let additions = add_cases();
    let divisions = div_cases();
    let cases = [
        (
            float_selector(&literals),
            (0..literals.len())
                .map(|n| n.to_string())
                .collect::<Vec<_>>(),
        ),
        (
            conversion(),
            conversion_cases()
                .iter()
                .map(|(n, _)| n.to_string())
                .collect(),
        ),
        (
            arithmetic_selector(0x1a, &additions),
            (0..additions.len()).map(|n| n.to_string()).collect(),
        ),
        (
            arithmetic_selector(0x1b, &divisions),
            (0..divisions.len()).map(|n| n.to_string()).collect(),
        ),
        (unused_invalid(0x1a), vec!["".into()]),
        (unused_invalid(0x1b), vec!["".into()]),
        (literal(1u64 << 63), vec!["".into(), "7".into()]),
        (float_loop(), vec!["".into()]),
        (
            program(
                &[8, 8],
                &[float(0, 0), op(0x55, 0, &[0]), op(0x1a, 8, &[1, 1, 1])],
            ),
            vec!["".into()],
        ),
    ];
    for profile in [
        BuildProfile::Baseline,
        BuildProfile::Size,
        BuildProfile::Speed,
    ] {
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
                        "case{index} {} input{input}",
                        profile.name()
                    ),
                    Err(reference) => {
                        let error = run_console_with_input(&artifact, input).unwrap_err();
                        assert_eq!(error.exit_code, Some(1));
                        for keyword in ["zero", "nonfinite", "unconsumed"] {
                            if reference.contains(keyword) {
                                assert!(
                                    error.stderr.contains(keyword),
                                    "{reference:?} vs{:?}",
                                    error.stderr
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
