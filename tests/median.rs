//! Binary partition source against an independent sorted-union oracle.
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

const SOURCE: &[u8] = include_bytes!("../examples/median-two-sorted-arrays.rvb");
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(native::unique_directory(&std::env::temp_dir(), "rivet-median").unwrap())
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn input(a: &[i64], b: &[i64]) -> String {
    [a, b]
        .iter()
        .map(|array| {
            let mut tokens = vec![array.len().to_string()];
            tokens.extend(array.iter().map(i64::to_string));
            tokens.join(" ")
        })
        .collect::<Vec<_>>()
        .join(" ")
        + "\n"
}
fn oracle(a: &[i64], b: &[i64]) -> f64 {
    let mut merged = [a, b].concat();
    merged.sort_unstable();
    let middle = merged.len() / 2;
    if merged.len() % 2 == 1 {
        merged[middle] as f64
    } else {
        (merged[middle - 1] as f64 + merged[middle] as f64) / 2.0
    }
}
fn reference(program: &language::CompiledProgram, data: &str) -> String {
    match program.execute_with_input(data).unwrap() {
        Execution::Console { stdout } => stdout,
        _ => panic!("median must return scalar output"),
    }
}

#[test]
fn median_matches_independent_oracle_and_standalone_all_profiles() {
    let program = language::compile_binary(SOURCE).unwrap();
    assert_eq!(program.encode_binary(), SOURCE);
    assert_eq!(program.instruction_count(), 81);
    assert_eq!(SOURCE.len(), 803);
    let mut cases = vec![
        (vec![1, 3], vec![2]),
        (vec![1, 2], vec![3, 4]),
        (vec![], vec![1]),
        (vec![2], vec![]),
        (vec![], vec![-2, 1]),
        (vec![-3, -2], vec![-1, 0]),
        (vec![0], vec![0]),
        (vec![-1_000_000], vec![1_000_000]),
        (vec![1, 1, 1], vec![1, 1]),
        (vec![1], vec![2, 3, 4, 5, 6]),
        (vec![1, 2, 3], vec![4, 5, 6]),
        (vec![4, 5, 6], vec![1, 2, 3]),
        ((0..1000).collect(), (1000..2000).collect()),
        ((1000..2000).collect(), (0..1000).collect()),
        (vec![-1_000_000; 1000], vec![1_000_000; 1000]),
        (vec![], (0..1000).collect()),
        ((0..1000).collect(), vec![]),
        (vec![1_000_000], (0..1000).collect()),
        (
            (0..1000).map(|x| x * 2).collect(),
            (0..1000).map(|x| x * 2 + 1).collect(),
        ),
    ];
    let mut seed = 0x4D45_4449_414E_u64;
    let mut next = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        seed >> 32
    };
    for index in 0..256 {
        let limit = if index % 4 == 0 { 1001 } else { 51 };
        let m = (next() % limit) as usize;
        let mut n = (next() % limit) as usize;
        if m + n == 0 {
            n = 1;
        }
        let mut a: Vec<i64> = (0..m)
            .map(|_| (next() % 2_000_001) as i64 - 1_000_000)
            .collect();
        let mut b: Vec<i64> = (0..n)
            .map(|_| (next() % 2_000_001) as i64 - 1_000_000)
            .collect();
        a.sort_unstable();
        b.sort_unstable();
        cases.push((a, b));
    }
    assert_eq!(cases.len(), 275);
    let temp = Temp::new();
    for profile in [
        BuildProfile::Baseline,
        BuildProfile::Size,
        BuildProfile::Speed,
    ] {
        let artifact =
            native::build_native(&program, &temp.0.join(profile.name()), profile).unwrap();
        let metadata: serde_json::Value =
            serde_json::from_slice(&fs::read(&artifact.metadata_path).unwrap()).unwrap();
        assert_eq!(
            metadata["optimization"],
            match profile {
                BuildProfile::Baseline => "-O0",
                BuildProfile::Size => "-Oz",
                BuildProfile::Speed => "-O3",
            }
        );
        assert_eq!(metadata["runtime_abi"], 2);
        for (a, b) in &cases {
            let data = input(a, b);
            let expected = reference(&program, &data);
            assert_eq!(
                expected.trim().parse::<f64>().unwrap(),
                oracle(a, b),
                "{data}"
            );
            let result = run_console_with_input(&artifact, &data).unwrap();
            assert_eq!(result.stdout, expected, "{}: {data}", profile.name());
            assert_eq!(result.exit_code, 0);
        }
        for data in [
            "",
            "0 0",
            "2 1",
            "1 +2 1 3",
            "1 2 1 3 7",
            "-1",
            "10001",
            "1 0 1 9223372036854775808",
            "1 0 1 1\0",
        ]
        .into_iter()
        .map(str::to_owned)
        .chain([input(&vec![0; 1001], &[1]), input(&[1], &vec![0; 1001])])
        {
            assert!(program.execute_with_input(&data).is_err(), "{data:?}");
            let error = run_console_with_input(&artifact, &data).unwrap_err();
            assert!(error.exit_code.is_some_and(|code| code != 0));
            assert!(!error.stderr.is_empty());
        }
        let folder = temp.0.join(format!("alone-{}", profile.name()));
        fs::create_dir(&folder).unwrap();
        let executable = folder.join("median");
        fs::copy(&artifact.path, &executable).unwrap();
        let mut child = Command::new(&executable)
            .current_dir(&folder)
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
            .write_all(b"2 1 2 2 3 4\n")
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"2.5000000000000000e+00\n");
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
    }
}

#[test]
fn exhaustive_small_sorted_inputs_and_cli_fractional_output() {
    fn arrays(prefix: &mut Vec<i64>, values: &mut Vec<Vec<i64>>) {
        values.push(prefix.clone());
        if prefix.len() == 3 {
            return;
        }
        for x in prefix.last().copied().unwrap_or(-2)..=2 {
            prefix.push(x);
            arrays(prefix, values);
            prefix.pop();
        }
    }
    let program = language::compile_binary(SOURCE).unwrap();
    let mut values = Vec::new();
    arrays(&mut Vec::new(), &mut values);
    for a in &values {
        for b in &values {
            if a.is_empty() && b.is_empty() {
                continue;
            }
            let actual = reference(&program, &input(a, b))
                .trim()
                .parse::<f64>()
                .unwrap();
            assert_eq!(actual, oracle(a, b), "{a:?} {b:?}");
        }
    }
    assert_eq!(values.len() * values.len() - 1, 3135);
    let mut child = Command::new(env!("CARGO_BIN_EXE_rivet"))
        .args(["interpret", "examples/median-two-sorted-arrays.rvb"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"0 2 -2 1\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"-5.0000000000000000e-01\n");
}
