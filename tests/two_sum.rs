//! Algorithm acceptance: bytes -> typed control flow -> real native executables.
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

const SOURCE: &[u8] = include_bytes!("../examples/two-sum.rvb");
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(native::unique_directory(&std::env::temp_dir(), "rivet-two-sum").unwrap())
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn input(nums: &[i64], target: i64) -> String {
    let mut values = vec![nums.len().to_string()];
    values.extend(nums.iter().map(i64::to_string));
    values.push(target.to_string());
    values.join(" ") + "\n"
}
fn pairs(nums: &[i64], target: i64) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    for i in 0..nums.len() {
        for j in i + 1..nums.len() {
            if nums[i] + nums[j] == target {
                found.push((i, j));
            }
        }
    }
    found
}
fn reference(program: &language::CompiledProgram, data: &str) -> String {
    match program.execute_with_input(data).unwrap() {
        Execution::Console { stdout } => stdout,
        _ => panic!("compute must produce console output"),
    }
}
fn assert_answer(nums: &[i64], target: i64, stdout: &str) {
    let answer: Vec<usize> = serde_json::from_str(stdout).unwrap();
    let expected = pairs(nums, target);
    if expected.is_empty() {
        assert!(answer.is_empty());
        return;
    }
    assert_eq!(answer.len(), 2);
    assert_ne!(answer[0], answer[1]);
    assert!(answer.iter().all(|i| *i < nums.len()));
    assert_eq!(nums[answer[0]] + nums[answer[1]], target);
}

#[test]
fn binary_two_sum_matches_oracle_and_native_profiles() {
    let program = language::compile_binary(SOURCE).unwrap();
    assert_eq!(program.encode_binary(), SOURCE);
    assert_eq!(program.instruction_count(), 21);
    let llvm = program.emit_llvm();
    assert!(llvm.contains("llvm.ssub.with.overflow.i64"));
    assert!(llvm.contains("@rv2_map_get"));
    assert!(llvm.contains("br i1"));
    // There is no specialized native solver call or embedded source dispatcher.
    assert!(!llvm.contains("@two_sum"));
    let mut cases = vec![
        (vec![2, 7, 11, 15], 9),
        (vec![3, 2, 4], 6),
        (vec![3, 3], 6),
        (vec![0, 4, 3, 0], 0),
        (vec![-3, 4, 3, 90], 0),
        (vec![-1_000_000_000, 1_000_000_000], 0),
        (vec![5, 5, 5, 1], 6),
        (vec![2, 7, 11, 15], 26),
        (vec![], 0),
        (vec![3], 6),
        (vec![1, 2, 4], 20),
    ];
    // Seeded unique-answer instances, checked by an independent quadratic oracle.
    let mut seed = 20261025u64;
    let mut next = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        seed >> 32
    };
    for _ in 0..10000 {
        let len = (next() % 23 + 2) as usize;
        let nums: Vec<i64> = (0..len).map(|_| (next() % 81) as i64 - 40).collect();
        let target = (next() % 121) as i64 - 60;
        if pairs(&nums, target).len() == 1 {
            cases.push((nums, target));
        }
        if cases.len() == 107 {
            break;
        }
    }
    assert_eq!(cases.len(), 107);
    let directory = Temp::new();
    for profile in [BuildProfile::Baseline, BuildProfile::Size] {
        let name = if matches!(profile, BuildProfile::Size) {
            "size"
        } else {
            "baseline"
        };
        let artifact = native::build_native(&program, &directory.0.join(name), profile).unwrap();
        let metadata: serde_json::Value =
            serde_json::from_slice(&fs::read(&artifact.metadata_path).unwrap()).unwrap();
        assert_eq!(metadata["runtime_abi"], 2);
        for (nums, target) in &cases {
            let data = input(nums, *target);
            let expected = reference(&program, &data);
            assert_answer(nums, *target, &expected);
            let actual = run_console_with_input(&artifact, &data).unwrap();
            assert_eq!(actual.stdout, expected, "input {data}");
            assert_eq!(actual.exit_code, 0);
        }
        let mut large: Vec<i64> = (0..9999).map(|i| -1_000_000_000 + i).collect();
        large.push(1_000_000_000);
        let data = input(&large, 9998);
        assert!(data.len() > 65536); // Exercise input larger than a pipe buffer.
        assert_eq!(reference(&program, &data), "[9998,9999]\n");
        assert_eq!(
            run_console_with_input(&artifact, &data).unwrap().stdout,
            "[9998,9999]\n"
        );
        for bad in [
            "",
            "2 3",
            "2 3 nope 6",
            "2 3 3 6 99",
            "-1 0",
            "10001",
            "2 +3 3 6",
            "2 9223372036854775808 3 6",
            "2 3 3 6\0",
            "2 -9223372036854775808 0 9223372036854775807",
        ] {
            assert!(program.execute_with_input(bad).is_err(), "{bad:?}");
            let error = run_console_with_input(&artifact, bad).unwrap_err();
            assert!(error.exit_code.is_some_and(|code| code != 0), "{error:?}");
            assert!(!error.stderr.is_empty());
        }
        assert!(run_console_with_input(&artifact, &" ".repeat(262145)).is_err());

        // Copy only the native executable into an otherwise empty directory.
        let standalone = directory.0.join(format!("independent-{name}"));
        fs::create_dir(&standalone).unwrap();
        let executable = standalone.join("solve");
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
            .write_all(b"3 3 2 4 6\n")
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "{:?}", output.stderr);
        assert_eq!(output.stdout, b"[1,2]\n");
        assert_eq!(fs::read_dir(&standalone).unwrap().count(), 1);
    }
}

#[test]
fn cli_accepts_runtime_data_separately_from_binary_source() {
    let directory = Temp::new();
    let source = directory.0.join("solution.rvb");
    fs::write(&source, SOURCE).unwrap();
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
        child.stdin.take().unwrap().write_all(b"2 3 3 6\n").unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"[0,1]\n");
    }
}

#[test]
fn compute_process_preserves_large_output_and_reaps_with_unread_stdin() {
    // Direct binary programs: generic array echo, and a jump-to-self loop.
    let echo = [
        0x52, 0x56, 0x42, 0, 2, 0, 0, 0, 2, 0, 3, 0, 17, 0, 0, 0, 1, 0, 5, 0x31, 5, 2, 0, 0, 0,
        0x52, 0, 2, 0, 0, 0, 0, 0,
    ];
    let forever = [
        0x52, 0x56, 0x42, 0, 2, 0, 0, 0, 1, 0, 3, 0, 11, 0, 0, 0, 1, 0, 2, 0x50, 0, 2, 0, 0, 0, 0,
        0,
    ];
    let echo = language::compile_binary(&echo).unwrap();
    let forever = language::compile_binary(&forever).unwrap();
    let data = "10000 ".to_owned() + &format!("{} ", i64::MIN).repeat(10000);
    let expected = reference(&echo, &data);
    assert_eq!(expected.len(), 210002);
    let directory = Temp::new();
    for (index, profile) in [BuildProfile::Baseline, BuildProfile::Size]
        .into_iter()
        .enumerate()
    {
        let artifact =
            native::build_native(&echo, &directory.0.join(format!("echo-{index}")), profile)
                .unwrap();
        assert_eq!(
            run_console_with_input(&artifact, &data).unwrap().stdout,
            expected
        );
        let artifact = native::build_native(
            &forever,
            &directory.0.join(format!("loop-{index}")),
            profile,
        )
        .unwrap();
        let start = std::time::Instant::now();
        let failure = run_console_with_input(&artifact, &"x".repeat(262144)).unwrap_err();
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
        assert_eq!(failure.exit_code, Some(1));
        assert!(failure.stderr.contains("1000000"), "{failure:?}");
    }
}
