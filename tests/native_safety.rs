//! Fuel optimization must preserve the exact instruction budget and error order.
use rivet::{
    language::{self, Execution},
    native::{self, BuildProfile},
    native_process::run_console_with_input,
};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

const LIMIT: usize = 1_000_000;
const NATIVE_FUEL_ERROR: &str = "Rivet runtime: compute instruction limit exceeded (1000000)\n";
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
fn float(destination: u16, value: f64) -> Vec<u8> {
    let mut payload = refs(&[destination]);
    payload.extend(value.to_bits().to_le_bytes());
    record(4, 8, &payload)
}
fn boolean(destination: u16, value: bool) -> Vec<u8> {
    let mut payload = refs(&[destination]);
    payload.push(value.into());
    record(3, 3, &payload)
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
fn output(source: &[u8], input: &str) -> Result<String, String> {
    match language::compile_binary(source)
        .unwrap()
        .execute_with_input(input)?
    {
        Execution::Console { stdout } => Ok(stdout),
        _ => panic!("compute output expected"),
    }
}
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        Self(native::unique_directory(&std::env::temp_dir(), "rivet-native-safety").unwrap())
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn profiles() -> [BuildProfile; 3] {
    [
        BuildProfile::Baseline,
        BuildProfile::Size,
        BuildProfile::Speed,
    ]
}

#[derive(Clone, Copy)]
enum Tail {
    Return,
    Assert,
    IntOverflow,
    FloatOverflow,
    FloatZero,
    ArrayBounds,
    ListNil,
    MissingInput,
    Allocation,
}
impl Tail {
    fn operation(self) -> Vec<u8> {
        match self {
            Self::Return => op(0x53, 0, &[3]),
            Self::Assert => op(0x54, 0, &[4]),
            Self::IntOverflow => op(0x11, 2, &[6, 6, 2]),
            Self::FloatOverflow => op(0x1a, 8, &[7, 7, 7]),
            Self::FloatZero => op(0x1b, 8, &[7, 7, 8]),
            Self::ArrayBounds => op(0x33, 2, &[6, 9, 1]),
            Self::ListNil => op(0x63, 2, &[6, 3]),
            Self::MissingInput => op(0x30, 2, &[6]),
            Self::Allocation => op(0x40, 6, &[10]),
        }
    }
    fn diagnostic(self) -> &'static str {
        match self {
            Self::Return => "",
            Self::Assert => "assertion failed",
            Self::IntOverflow => "signed 64-bit integer overflow",
            Self::FloatOverflow => "nonfinite floating-point result",
            Self::FloatZero => "floating-point division by zero",
            Self::ArrayBounds => "array index out of bounds",
            Self::ListNil => "nil",
            Self::MissingInput => "missing an integer",
            Self::Allocation => "compute allocation budget exceeds 67108864 bytes",
        }
    }
}

fn boundary_program(tail: Tail, tail_step: usize) -> (Vec<u8>, String) {
    let mut operations = vec![
        op(0x30, 2, &[0]),
        int(1, 0),
        int(2, 1),
        op(0x60, 7, &[3]),
        boolean(4, false),
        int(6, i64::MAX),
        float(7, f64::MAX),
        float(8, -0.),
        op(0x34, 5, &[9, 0]),
    ];
    if matches!(tail, Tail::Allocation) {
        // 85 maps leave 262144 logical bytes; the next map costs786432 and fails.
        operations.extend(vec![op(0x40, 6, &[10]); 85]);
    }
    let setup = operations.len();
    let iterations = (tail_step - setup - 3) / 4;
    let padding = tail_step - (setup + 4 * iterations + 2) - 1;
    assert!(padding < 4);
    let head = operations.len() as u16;
    operations.extend([
        op(0x14, 3, &[5, 0, 1]),
        op(0x51, 0, &[5, head + 4, head + 2]),
        op(0x13, 2, &[0, 0, 2]),
        op(0x50, 0, &[head]),
    ]);
    operations.extend(vec![op(0x16, 2, &[1, 1]); padding]);
    operations.push(tail.operation());
    if !matches!(tail, Tail::Return) {
        operations.push(op(0x53, 0, &[3]));
    }
    // setup + (4 * iterations + 2) loop visits + padding + tail = tail_step.
    (
        program(&[2, 2, 2, 7, 3, 3, 2, 8, 8, 5, 6], &operations),
        iterations.to_string(),
    )
}

#[test]
fn source_tick_sites_remain_exact_and_inline_body_uses_the_stable_counter_contract() {
    for source in [
        include_bytes!("../examples/two-sum.rvb").as_slice(),
        include_bytes!("../examples/add-two-numbers.rvb").as_slice(),
        include_bytes!("../examples/median-two-sorted-arrays.rvb").as_slice(),
    ] {
        let compiled = language::compile_binary(source).unwrap();
        let llvm = compiled.emit_llvm();
        assert_eq!(
            llvm.matches("call void @rv2_tick()").count(),
            compiled.instruction_count()
        );
        assert!(llvm.contains("define available_externally void @rv2_tick() alwaysinline"));
        assert!(llvm.contains("declare ptr @rv2_fuel_counter() nounwind willreturn memory(none)"));
        assert!(llvm.contains("icmp eq i64 %steps, 1000000"));
        assert!(llvm.contains("%next = add i64 %steps, 1"));
        assert!(llvm.contains("store i64 %next, ptr %counter"));
        assert!(llvm.contains("compute instruction limit exceeded (1000000)"));
        assert!(!llvm.contains("@rv2_steps")); // No runtime layout/private-global coupling.
    }
}

#[test]
fn exact_last_allowed_instruction_returns_and_first_disallowed_instruction_fails() {
    let scratch = Scratch::new();
    let cases = [LIMIT - 1, LIMIT, LIMIT + 1].map(|step| {
        let (source, input) = boundary_program(Tail::Return, step);
        let expected = output(&source, &input);
        if step <= LIMIT {
            assert_eq!(expected.as_deref(), Ok("[]\n"));
        } else {
            assert!(expected.unwrap_err().contains("1000000"));
        }
        (source, input, step)
    });
    for profile in profiles() {
        for (source, input, step) in &cases {
            let compiled = language::compile_binary(source).unwrap();
            let artifact = native::build_native(
                &compiled,
                &scratch.0.join(format!("{}-{step}", profile.name())),
                profile,
            )
            .unwrap();
            if *step <= LIMIT {
                assert_eq!(
                    run_console_with_input(&artifact, input).unwrap().stdout,
                    "[]\n"
                );
            } else {
                let error = run_console_with_input(&artifact, input).unwrap_err();
                assert_eq!(error.exit_code, Some(1));
                assert_eq!(error.stderr, NATIVE_FUEL_ERROR);
            }
        }
    }
}

#[test]
fn language_errors_at_the_limit_win_but_the_next_operation_fails_on_fuel_first() {
    let scratch = Scratch::new();
    let mut cases = Vec::new();
    for tail in [
        Tail::Assert,
        Tail::IntOverflow,
        Tail::FloatOverflow,
        Tail::FloatZero,
        Tail::ArrayBounds,
        Tail::ListNil,
        Tail::MissingInput,
        Tail::Allocation,
    ] {
        for step in [LIMIT, LIMIT + 1] {
            let (source, input) = boundary_program(tail, step);
            let reference = output(&source, &input).unwrap_err();
            if step == LIMIT {
                assert!(reference.contains(tail.diagnostic()), "{reference}");
            } else {
                assert!(reference.contains("1000000"), "{reference}");
            }
            cases.push((source, input, step, tail.diagnostic()));
        }
    }
    // Also cover a semantic failure immediately before the last allowed tick.
    let (source, input) = boundary_program(Tail::Assert, LIMIT - 1);
    assert!(
        output(&source, &input)
            .unwrap_err()
            .contains("assertion failed")
    );
    cases.push((source, input, LIMIT - 1, "assertion failed"));
    for profile in profiles() {
        for (index, (source, input, step, diagnostic)) in cases.iter().enumerate() {
            let compiled = language::compile_binary(source).unwrap();
            let artifact = native::build_native(
                &compiled,
                &scratch.0.join(format!("{}-{index}", profile.name())),
                profile,
            )
            .unwrap();
            let error = run_console_with_input(&artifact, input).unwrap_err();
            assert_eq!(error.exit_code, Some(1));
            if *step <= LIMIT {
                assert!(
                    error.stderr.contains(diagnostic),
                    "{} case{index}: {:?}",
                    profile.name(),
                    error.stderr
                );
                assert_ne!(error.stderr, NATIVE_FUEL_ERROR);
            } else {
                assert_eq!(
                    error.stderr,
                    NATIVE_FUEL_ERROR,
                    "{} case{index}",
                    profile.name()
                );
            }
        }
    }
}

fn checked(command: &mut Command) -> Output {
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "{command:?}: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    result
}

#[test]
fn inline_and_real_c_ticks_share_state_and_cleanup_under_every_profile() {
    let scratch = Scratch::new();
    let source = program(&[7], &[op(0x60, 7, &[0]), op(0x53, 0, &[0])]);
    assert_eq!(output(&source, "").unwrap(), "[]\n");
    let original = language::compile_binary(&source).unwrap().emit_llvm();
    // Test-only entry/cleanup observation; generated instruction calls/operations
    // are untouched. The production emitter and benchmark are not modified.
    let inline = original
        .replace("@main()", "@rv_test_program()")
        .replace("@rv_cleanup", "@test_cleanup");
    let start = inline
        .find("define available_externally void @rv2_tick()")
        .unwrap();
    let end = start + inline[start..].find("\n}\n").unwrap() + 3;
    let mut fallback = inline.clone();
    fallback.replace_range(start..end, "declare void @rv2_tick()\n");
    let harness = r#"#include "runtime.h"
#include <stdint.h>
#include <stdio.h>
#include <string.h>
extern int rv_test_program(void);
static int mode;
static unsigned calls;
static uint64_t seen;
void test_cleanup(void) {
    seen = *rv2_fuel_counter();
    ++calls;
    if (mode == 2) rv2_tick(); /* C fallback after final inline tick must fail. */
    rv_cleanup();
}
int main(int argc, char **argv) {
    uint64_t *first = rv2_fuel_counter();
    if (first == NULL || *first != 0 || first != rv2_fuel_counter()) return 20;
    if (argc > 1 && strcmp(argv[1], "inline-after-c") == 0) {
        mode = 1;
        *first = 999998;
        rv2_tick();
        return rv_test_program(); /* Inline return's tick is the first forbidden one. */
    }
    if (argc > 1 && strcmp(argv[1], "c-after-inline") == 0) {
        mode = 2;
        *first = 999998;
        return rv_test_program(); /* Cleanup callback invokes C at1000000. */
    }
    rv2_tick();
    if (*first != 1 || rv_test_program() != 0 || seen != 3 || calls != 1) return 21;
    if (first != rv2_fuel_counter() || *first != 0) return 22;
    rv2_tick(); rv2_tick();
    if (*first != 2) return 23;
    rv_cleanup();
    if (first != rv2_fuel_counter() || *first != 0) return 24;
    puts("shared-counter-ok");
    return 0;
}
"#;
    fs::write(
        scratch.0.join("runtime.c"),
        include_str!("../native/runtime.c"),
    )
    .unwrap();
    fs::write(
        scratch.0.join("runtime.h"),
        include_str!("../native/runtime.h"),
    )
    .unwrap();
    fs::write(scratch.0.join("harness.c"), harness).unwrap();
    fs::write(scratch.0.join("inline.ll"), inline).unwrap();
    fs::write(scratch.0.join("fallback.ll"), fallback).unwrap();
    let clang = std::env::var_os("RIVET_CLANG").unwrap_or_else(|| "clang".into());
    for profile in profiles() {
        let optimization = match profile {
            BuildProfile::Baseline => "-O0",
            BuildProfile::Size => "-Oz",
            BuildProfile::Speed => "-O3",
        };
        let runtime = scratch.0.join(format!("runtime-{}.o", profile.name()));
        let harness = scratch.0.join(format!("harness-{}.o", profile.name()));
        checked(
            Command::new(&clang)
                .args([
                    "-std=c11",
                    "-D_POSIX_C_SOURCE=200809L",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    optimization,
                    "-c",
                ])
                .arg(scratch.0.join("runtime.c"))
                .arg("-o")
                .arg(&runtime),
        );
        checked(
            Command::new(&clang)
                .args([
                    "-std=c11",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    optimization,
                    "-c",
                ])
                .arg(scratch.0.join("harness.c"))
                .arg("-o")
                .arg(&harness),
        );
        for variant in ["inline", "fallback"] {
            let object = scratch.0.join(format!("{variant}-{}.o", profile.name()));
            let executable = scratch.0.join(format!("{variant}-{}", profile.name()));
            checked(
                Command::new(&clang)
                    .args([optimization, "-c"])
                    .arg(scratch.0.join(format!("{variant}.ll")))
                    .arg("-o")
                    .arg(&object),
            );
            checked(
                Command::new(&clang)
                    .arg(&object)
                    .arg(&runtime)
                    .arg(&harness)
                    .arg("-o")
                    .arg(&executable),
            );
            let normal = checked(&mut Command::new(&executable));
            assert_eq!(normal.stdout, b"[]\nshared-counter-ok\n");
            for mode in ["inline-after-c", "c-after-inline"] {
                let failed = Command::new(&executable).arg(mode).output().unwrap();
                assert_eq!(failed.status.code(), Some(1));
                assert_eq!(failed.stderr, NATIVE_FUEL_ERROR.as_bytes());
                assert_eq!(
                    failed.stdout,
                    if mode == "inline-after-c" {
                        b"".as_slice()
                    } else {
                        b"[]\n".as_slice()
                    }
                );
            }
        }
    }
}
