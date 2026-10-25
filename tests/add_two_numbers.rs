//! Actual binary list algorithm against a separate arbitrary-precision oracle.
use rivet::{
    language::{self, Execution},
    native::{self, BuildProfile},
    native_process::run_console_with_input,
};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

const SOURCE: &[u8] = include_bytes!("../examples/add-two-numbers.rvb");
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(native::unique_directory(&std::env::temp_dir(), "rivet-add-numbers").unwrap())
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn input(left: &[i64], right: &[i64]) -> String {
    [left, right]
        .iter()
        .map(|list| {
            let mut tokens = vec![list.len().to_string()];
            tokens.extend(list.iter().map(i64::to_string));
            tokens.join(" ")
        })
        .collect::<Vec<_>>()
        .join(" ")
        + "\n"
}
fn oracle(cases: &[(Vec<i64>, Vec<i64>)]) -> Vec<Vec<i64>> {
    // Test data only: Python arbitrary-precision integers provide an independent
    // oracle, without reimplementing the Rivet node/carry algorithm in Rust.
    let mut child = Command::new("python3")
        .args([
            "-c",
            r#"
import json, sys
def number(digits):
    return int(''.join(str(d) for d in reversed(digits)))
print(json.dumps([[int(c) for c in str(number(a) + number(b))[::-1]]
                  for a, b in json.load(sys.stdin)]))
"#,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(cases).unwrap())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}
fn reference(program: &language::CompiledProgram, data: &str) -> String {
    match program.execute_with_input(data).unwrap() {
        Execution::Console { stdout } => stdout,
        _ => panic!("list computation must produce console output"),
    }
}

#[test]
fn binary_linked_list_addition_matches_big_integer_oracle_both_profiles() {
    let program = language::compile_binary(SOURCE).unwrap();
    assert_eq!(program.encode_binary(), SOURCE);
    assert_eq!(program.instruction_count(), 70);
    let llvm = program.emit_llvm();
    for operation in [
        "@rv2_read_list",
        "@rv2_list_cons",
        "@rv2_list_next",
        "@rv2_return_list",
        "sdiv i64",
        "srem i64",
    ] {
        assert!(
            llvm.contains(operation),
            "missing native operation {operation}"
        );
    }
    let mut cases = vec![
        (vec![2, 4, 3], vec![5, 6, 4]),
        (vec![0], vec![0]),
        (vec![9; 7], vec![9; 4]),
        (vec![9], vec![1]),
        (vec![1], vec![9]),
        (vec![1], vec![9, 9]),
        (vec![5, 1], vec![5]),
        (vec![0], vec![1]),
        (vec![1], vec![0]),
        (vec![0, 1], vec![0, 9]),
        (vec![9; 100], vec![1]),
        (vec![9; 100], vec![9; 100]),
        (vec![1], vec![9; 100]),
        (
            [vec![0; 99], vec![1]].concat(),
            [vec![0; 99], vec![1]].concat(),
        ),
    ];
    let mut seed = 0xADD2_2026_1025u64;
    let mut next = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        seed >> 32
    };
    for _ in 0..256 {
        let mut lists = Vec::new();
        for _ in 0..2 {
            let len = (next() % 100 + 1) as usize;
            let mut digits: Vec<i64> = (0..len).map(|_| (next() % 10) as i64).collect();
            if len > 1 {
                digits[len - 1] = (next() % 9 + 1) as i64;
            }
            lists.push(digits);
        }
        cases.push((lists.remove(0), lists.remove(0)));
    }
    assert_eq!(cases.len(), 270);
    let answers = oracle(&cases);
    assert_eq!(answers[0], [7, 0, 8]);
    assert_eq!(answers[1], [0]);
    assert_eq!(answers[2], [8, 9, 9, 9, 0, 0, 0, 1]);
    assert_eq!(answers[10], [vec![0; 100], vec![1]].concat());
    let directory = Temp::new();
    for (index, profile) in [BuildProfile::Baseline, BuildProfile::Size]
        .into_iter()
        .enumerate()
    {
        let artifact = native::build_native(
            &program,
            &directory.0.join(format!("solve-{index}")),
            profile,
        )
        .unwrap();
        for ((left, right), answer) in cases.iter().zip(&answers) {
            let data = input(left, right);
            let expected = serde_json::to_string(answer).unwrap() + "\n";
            assert_eq!(reference(&program, &data), expected, "input {data}");
            let result = run_console_with_input(&artifact, &data).unwrap();
            assert_eq!(result.stdout, expected, "input {data}");
            assert_eq!(result.exit_code, 0);
        }
        for data in [
            "",
            "0 1 0",
            "1 0 0",
            "1 10 1 0",
            "1 -1 1 0",
            "2 1 0 1 0",
            "1 0 2 1 0",
            "1 3 1 4 99",
            "1 +3 1 4",
            "1 3 1",
            "10001",
            "1 9223372036854775808 1 0",
            "1 3 1 4\0",
        ]
        .into_iter()
        .map(str::to_owned)
        .chain([input(&[1; 101], &[0]), input(&[0], &[1; 101])])
        {
            assert!(program.execute_with_input(&data).is_err(), "{data:?}");
            let error = run_console_with_input(&artifact, &data).unwrap_err();
            assert!(error.exit_code.is_some_and(|code| code != 0), "{error:?}");
            assert!(!error.stderr.is_empty());
        }
        // Only a copied executable is present in this directory; no runtime/source.
        let standalone = directory.0.join(format!("standalone-{index}"));
        fs::create_dir(&standalone).unwrap();
        let executable = standalone.join("add");
        fs::copy(&artifact.path, &executable).unwrap();
        let mut child = Command::new(&executable)
            .current_dir(&standalone)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"3 2 4 3 3 5 6 4\n")
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(result.status.success(), "{:?}", result.stderr);
        assert_eq!(result.stdout, b"[7,0,8]\n");
        assert_eq!(fs::read_dir(&standalone).unwrap().count(), 1);
    }
}

#[test]
fn cli_reads_two_lists_and_returns_a_101_digit_sum() {
    let directory = Temp::new();
    let source = directory.0.join("addition.rvb");
    fs::write(&source, SOURCE).unwrap();
    let data = input(&[9; 100], &[1]);
    let expected = serde_json::to_string(&[vec![0; 100], vec![1]].concat()).unwrap() + "\n";
    for command in ["interpret", "run"] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rivet"))
            .current_dir(&directory.0)
            .arg(command)
            .arg(&source)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(data.as_bytes())
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(result.stdout, expected.as_bytes());
    }
}
