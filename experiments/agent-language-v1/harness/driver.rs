//! Generic safe-Rust I/O and measurement driver; the external crate owns every solver.
#![forbid(unsafe_code)]

use std::{
    env,
    fs::File,
    hint::black_box,
    io::{self, BufRead, BufReader, BufWriter, Write},
    time::Instant,
};

#[cfg(not(any(task = "two_sum", task = "add_two_numbers", task = "median")))]
compile_error!("select --cfg task=\"two_sum\", task=\"add_two_numbers\", or task=\"median\"");
#[cfg(any(
    all(task = "two_sum", task = "add_two_numbers"),
    all(task = "two_sum", task = "median"),
    all(task = "add_two_numbers", task = "median")
))]
compile_error!("select exactly one task");

const MAX_VALUES: usize = 10_000;
const ANSWER_CAPACITY: usize = 10_002;
const MAX_CASES: usize = 256;
const MAX_INPUT_BYTES: usize = 128 * 1024 * 1024;
const MAX_CALLS: u64 = 100_000_000;
const MAX_WARMUPS: u64 = 10_000;
const HASH_SEED: u64 = 1_469_598_103_934_665_603;
const HASH_MULTIPLIER: u64 = 1_099_511_628_211;
type Result<T> = std::result::Result<T, String>;

// Stream tokens so pool text does not create a language-specific retained copy.
struct Scanner<R> {
    reader: R,
    bytes: usize,
}
impl<R: BufRead> Scanner<R> {
    fn new(reader: R) -> Self {
        Self { reader, bytes: 0 }
    }
    fn byte(&mut self) -> Result<Option<u8>> {
        let buffer = self.reader.fill_buf().map_err(|e| e.to_string())?;
        if buffer.is_empty() {
            return Ok(None);
        }
        let byte = buffer[0];
        self.reader.consume(1);
        self.bytes += 1;
        if self.bytes > MAX_INPUT_BYTES {
            return Err("input exceeds 128 MiB".into());
        }
        Ok(Some(byte))
    }
    fn nonspace(&mut self) -> Result<Option<u8>> {
        while let Some(byte) = self.byte()? {
            if !matches!(byte, b' ' | b'\t'..=b'\r') {
                return Ok(Some(byte));
            }
        }
        Ok(None)
    }
    fn integer(&mut self) -> Result<i64> {
        let first = self.nonspace()?.ok_or("missing integer")?;
        let negative = first == b'-';
        let limit = if negative {
            1u64 << 63
        } else {
            i64::MAX as u64
        };
        let mut current = if negative { self.byte()? } else { Some(first) };
        let mut magnitude = 0u64;
        let mut digits = 0usize;
        while let Some(byte) = current {
            if matches!(byte, b' ' | b'\t'..=b'\r') {
                break;
            }
            if !byte.is_ascii_digit() {
                return Err("invalid integer token".into());
            }
            let digit = u64::from(byte - b'0');
            if magnitude > (limit - digit) / 10 {
                return Err("integer outside signed 64-bit range".into());
            }
            magnitude = magnitude * 10 + digit;
            digits += 1;
            current = self.byte()?;
        }
        if digits == 0 {
            return Err("invalid integer token".into());
        }
        if negative && magnitude == 1u64 << 63 {
            Ok(i64::MIN)
        } else if negative {
            Ok(-(magnitude as i64))
        } else {
            Ok(magnitude as i64)
        }
    }
    fn count(&mut self, maximum: usize) -> Result<usize> {
        let value = self.integer()?;
        let count = usize::try_from(value).map_err(|_| "negative count")?;
        if count > maximum {
            return Err(format!("count exceeds {maximum}"));
        }
        Ok(count)
    }
    fn values(&mut self, maximum: usize) -> Result<Vec<i64>> {
        let count = self.count(maximum)?;
        (0..count).map(|_| self.integer()).collect()
    }
    fn finish(&mut self) -> Result<()> {
        if self.nonspace()?.is_some() {
            return Err("unexpected trailing input".into());
        }
        Ok(())
    }
}

#[cfg(task = "two_sum")]
struct Input {
    values: Vec<i64>,
    target: i64,
}
#[cfg(task = "median")]
struct Input {
    first: Vec<i64>,
    second: Vec<i64>,
}
#[cfg(task = "add_two_numbers")]
struct Input {
    first: Option<Box<solution::Node>>,
    second: Option<Box<solution::Node>>,
}

#[cfg(task = "add_two_numbers")]
fn list(values: Vec<i64>) -> Option<Box<solution::Node>> {
    let mut next = None;
    for value in values.into_iter().rev() {
        next = Some(Box::new(solution::Node { value, next }));
    }
    next
}
#[cfg(task = "add_two_numbers")]
fn free_list(list: &mut Option<Box<solution::Node>>) {
    while let Some(mut node) = list.take() {
        *list = node.next.take();
    }
}
#[cfg(task = "add_two_numbers")]
impl Drop for Input {
    fn drop(&mut self) {
        free_list(&mut self.first);
        free_list(&mut self.second);
    }
}

impl Input {
    fn read<R: BufRead>(scanner: &mut Scanner<R>) -> Result<Self> {
        #[cfg(task = "two_sum")]
        {
            Ok(Self {
                values: scanner.values(MAX_VALUES)?,
                target: scanner.integer()?,
            })
        }
        #[cfg(task = "median")]
        {
            Ok(Self {
                first: scanner.values(MAX_VALUES)?,
                second: scanner.values(MAX_VALUES)?,
            })
        }
        #[cfg(task = "add_two_numbers")]
        {
            // Parse before construction: malformed second input cannot leave a
            // long first list to be recursively dropped during an early return.
            let first = scanner.values(MAX_VALUES)?;
            let second = scanner.values(MAX_VALUES)?;
            Ok(Self {
                first: list(first),
                second: list(second),
            })
        }
    }
    fn capture(&self, answer: &mut Answer) -> Result<()> {
        answer.values.clear();
        #[cfg(task = "two_sum")]
        {
            let (first, second) = black_box(solution::entry(
                black_box(self.values.as_slice()),
                black_box(self.target),
            ));
            answer
                .values
                .push(i64::try_from(first).map_err(|_| "index exceeds i64")?);
            answer
                .values
                .push(i64::try_from(second).map_err(|_| "index exceeds i64")?);
            answer.values.sort_unstable();
        }
        #[cfg(task = "median")]
        {
            let result = black_box(solution::entry(
                black_box(self.first.as_slice()),
                black_box(self.second.as_slice()),
            ));
            let twice = result * 2.0;
            if !result.is_finite()
                || !twice.is_finite()
                || twice.fract() != 0.0
                || twice < i64::MIN as f64
                || twice >= -(i64::MIN as f64)
            {
                return Err(
                    "median is not a finite half-integer representable as doubled i64".into(),
                );
            }
            answer.scalar = result;
            answer.values.push(twice as i64);
        }
        #[cfg(task = "add_two_numbers")]
        {
            answer.owned = black_box(solution::entry(
                black_box(&self.first),
                black_box(&self.second),
            ));
            let mut current = answer.owned.as_deref();
            while let Some(node) = current {
                if answer.values.len() == ANSWER_CAPACITY {
                    return Err("result exceeds answer capacity".into());
                }
                answer.values.push(node.value);
                current = node.next.as_deref();
            }
        }
        Ok(())
    }
}

struct Answer {
    values: Vec<i64>,
    #[cfg(task = "median")]
    scalar: f64,
    #[cfg(task = "add_two_numbers")]
    owned: Option<Box<solution::Node>>,
}
impl Answer {
    fn new() -> Self {
        Self {
            values: Vec::with_capacity(ANSWER_CAPACITY),
            #[cfg(task = "median")]
            scalar: 0.0,
            #[cfg(task = "add_two_numbers")]
            owned: None,
        }
    }
    fn release(&mut self) {
        #[cfg(task = "add_two_numbers")]
        free_list(&mut self.owned);
    }
    fn print_json(&self, output: &mut impl Write) -> Result<()> {
        #[cfg(task = "median")]
        {
            writeln!(output, "{:.16e}", self.scalar).map_err(|e| e.to_string())?;
        }
        #[cfg(not(task = "median"))]
        {
            write!(output, "[").map_err(|e| e.to_string())?;
            for (index, value) in self.values.iter().enumerate() {
                if index != 0 {
                    write!(output, ",").map_err(|e| e.to_string())?;
                }
                write!(output, "{value}").map_err(|e| e.to_string())?;
            }
            writeln!(output, "]").map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
impl Drop for Answer {
    fn drop(&mut self) {
        self.release();
    }
}
fn hash(values: &[i64]) -> u64 {
    let mut hash = (HASH_SEED ^ values.len() as u64).wrapping_mul(HASH_MULTIPLIER);
    for value in values {
        hash = (hash ^ *value as u64).wrapping_mul(HASH_MULTIPLIER);
    }
    hash
}

struct Case {
    input: Input,
    expected: Vec<i64>,
    expected_hash: u64,
}
fn load_pool(path: &str) -> Result<Vec<Case>> {
    let mut scanner = Scanner::new(BufReader::new(File::open(path).map_err(|e| e.to_string())?));
    let count = scanner.count(MAX_CASES)?;
    if count == 0 {
        return Err("pool must contain at least one case".into());
    }
    let mut cases = Vec::with_capacity(count);
    for _ in 0..count {
        let input = Input::read(&mut scanner)?;
        let expected = scanner.values(ANSWER_CAPACITY)?;
        #[cfg(task = "two_sum")]
        let expected = {
            if expected.len() != 2 {
                return Err("Two Sum expectation must contain two indices".into());
            }
            let mut expected = expected;
            expected.sort_unstable();
            expected
        };
        #[cfg(task = "median")]
        if expected.len() != 1 {
            return Err("median expectation must contain one doubled value".into());
        }
        let expected_hash = hash(&expected);
        cases.push(Case {
            input,
            expected,
            expected_hash,
        });
    }
    scanner.finish()?;
    Ok(cases)
}

fn preflight(cases: &[Case], answer: &mut Answer) -> Result<()> {
    for (index, case) in cases.iter().enumerate() {
        let captured = case.input.capture(answer);
        let matches = captured.is_ok() && answer.values == case.expected;
        answer.release();
        captured?;
        if !matches {
            return Err(format!("preflight case {index} answer mismatch"));
        }
    }
    Ok(())
}
#[derive(Clone, Copy)]
enum Mode {
    Evaluate,
    Control,
}
impl Mode {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "evaluate" => Ok(Self::Evaluate),
            "control" => Ok(Self::Control),
            _ => Err("mode must be evaluate or control".into()),
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Evaluate => "evaluate",
            Self::Control => "control",
        }
    }
}
fn rounds(cases: &[Case], answer: &mut Answer, rounds: u64, mode: Mode) -> Result<u64> {
    let mut checksum = 0u64;
    for _ in 0..rounds {
        for (index, case) in cases.iter().enumerate() {
            let answer_hash = match mode {
                Mode::Evaluate => {
                    let captured = case.input.capture(answer);
                    let hash = if captured.is_ok() {
                        hash(&answer.values)
                    } else {
                        0
                    };
                    answer.release();
                    captured?;
                    hash
                }
                Mode::Control => black_box(case.expected_hash),
            };
            checksum = checksum.wrapping_add(answer_hash.wrapping_mul(1 + (index % 17) as u64));
        }
    }
    Ok(black_box(checksum))
}
fn unsigned(value: &str) -> Result<u64> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err("invalid unsigned benchmark argument".into());
    }
    value
        .parse()
        .map_err(|_| "benchmark argument exceeds u64".into())
}
fn benchmark(path: &str, round_arg: &str, warmup_arg: &str, mode: &str) -> Result<()> {
    let iterations = unsigned(round_arg)?;
    let warmups = unsigned(warmup_arg)?;
    let mode = Mode::parse(mode)?;
    if iterations == 0 {
        return Err("rounds must be positive".into());
    }
    if warmups > MAX_WARMUPS {
        return Err("warmups exceed 10000".into());
    }
    let cases = load_pool(path)?;
    let calls = iterations
        .checked_mul(cases.len() as u64)
        .ok_or("timed call count overflows")?;
    if calls > MAX_CALLS {
        return Err("timed calls exceed 100000000".into());
    }
    let expected_per_round = cases.iter().enumerate().fold(0u64, |total, (index, case)| {
        total.wrapping_add(case.expected_hash.wrapping_mul(1 + (index % 17) as u64))
    });
    let mut answer = Answer::new();
    preflight(&cases, &mut answer)?;
    let warmup_checksum = rounds(&cases, &mut answer, warmups, mode)?;
    if warmup_checksum != expected_per_round.wrapping_mul(warmups) {
        return Err("warmup checksum mismatch".into());
    }
    let started = Instant::now();
    let checksum = rounds(&cases, &mut answer, iterations, mode)?;
    let elapsed_ns = started.elapsed().as_nanos();
    if checksum != expected_per_round.wrapping_mul(iterations) {
        return Err("timed checksum mismatch".into());
    }
    let count = cases.len();
    let warmup_calls = warmups * count as u64;
    // Real retained-input cleanup happens after timing, before report output.
    drop(answer);
    drop(cases);
    let zeros = vec!["0"; count].join(",");
    println!(
        "{{\"mode\":\"{}\",\"cases\":{count},\"rounds\":{iterations},\"calls\":{calls},\"warmup_calls\":{warmup_calls},\"elapsed_ns\":{elapsed_ns},\"checksum\":{checksum},\"expected_checksum_per_round\":{expected_per_round},\"ticks_per_case\":[{zeros}]}}",
        mode.name()
    );
    Ok(())
}
fn run() -> Result<()> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() == 1 {
        let stdin = io::stdin();
        let mut scanner = Scanner::new(stdin.lock());
        let input = Input::read(&mut scanner)?;
        scanner.finish()?;
        let mut answer = Answer::new();
        input.capture(&mut answer)?;
        let stdout = io::stdout();
        let mut output = BufWriter::new(stdout.lock());
        answer.print_json(&mut output)?;
        answer.release();
        output.flush().map_err(|e| e.to_string())?;
        Ok(())
    } else if args.len() == 6 && args[1] == "--bench" {
        benchmark(&args[2], &args[3], &args[4], &args[5])
    } else {
        Err("usage: program [--bench POOL ROUNDS WARMUPS evaluate|control]".into())
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("Rust harness: {error}");
        std::process::exit(1);
    }
}
