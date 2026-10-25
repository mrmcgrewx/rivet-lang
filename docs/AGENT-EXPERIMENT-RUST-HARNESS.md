# Rust experiment harness preparation

Status: implemented and checked as **unmeasured preparation**, against the interfaces in [AGENT-EXPERIMENT-PLAN.md](AGENT-EXPERIMENT-PLAN.md). This work supplies generic input, output, ownership and timing code. It contains no Two Sum, addition or median solver, and supplies no evidence about agent authoring quality or solver performance.

Ownership is limited to this document and `experiments/agent-language-v1/harness/driver.rs`. The coordinator owns the experiment runner, frozen corpora, trial solvers, publication and Git. No production files or existing examples changed.

## Compilation and entry interface

The driver selects exactly one of `--cfg 'task="two_sum"'`, `--cfg 'task="add_two_numbers"'` and `--cfg 'task="median"'`. It links an external `solution` rlib, calling its public `entry` with the contract's solver signature. The addition crate must expose `Node { pub value: i64, pub next: Option<Box<Node>> }` at its root. The coordinator appends the non-inlined entry wrapper; the driver does not define or modify the solver.

Both compilation steps use Rust 1.90, edition 2024, optimization level 3, checked integer overflow, aborting panics, no LTO, forbidden unsafe code and warnings as errors. For example, with the prepared crate and output paths supplied by the coordinator:

```sh
rustc --edition=2024 --crate-name solution --crate-type rlib \
  -C opt-level=3 -C overflow-checks=yes -C panic=abort -C lto=off \
  -Funsafe_code -Dwarnings prepared.rs -o libsolution.rlib
rustc --edition=2024 --cfg 'task="two_sum"' \
  --extern solution=libsolution.rlib \
  -C opt-level=3 -C overflow-checks=yes -C panic=abort -C lto=off \
  -Funsafe_code -Dwarnings experiments/agent-language-v1/harness/driver.rs \
  -o program
```

## Boundaries and measurement

With no arguments, the executable reads one input from stdin and emits a JSON array or scalar. Its streaming parser accepts ASCII whitespace and optional minus signs, checks the full signed 64-bit range, and rejects trailing non-whitespace. Input collection lengths are bounded at 10,000; output capture is bounded at 10,002 values. The shared preparation limit is 128 MiB of input text.

`--bench POOL ROUNDS WARMUPS MODE` follows the frozen pool framing and report fields. There must be 1–256 cases, positive rounds, at most 100,000,000 timed calls and at most 10,000 warmup rounds. `MODE` is `evaluate` or `control`. Both modes first execute every solver input and compare the complete normalized answer with the expectation. Two Sum pairs are sorted; median outputs must be finite half-integers whose doubled value fits signed 64-bit. The full answer comparison happens before timing. Warmup and timed checksums are checked against their wrapping expected totals.

The timed `Instant` interval includes each solver invocation, output capture and normalization, answer hashing, weighted checksum accumulation, and release of each solver-owned result. `black_box` wraps solver arguments and results; control mode uses a black-box expected hash. Every invocation reuses a single `Vec<i64>` capture buffer reserved to the maximum capacity before preflight. Successful queries do not allocate a fresh harness capture buffer. The solver may allocate its own data structures and result nodes.

Parsing, expected-answer storage, retained input construction, capture-buffer allocation, preflight and warmup precede timing. Input teardown, formatting and report output follow it. Seven samples, randomized ordering, repetition calibration, child RSS and CPU collection belong to the coordinator. Rust reports one zero in `ticks_per_case` per case. Control measurements are separate observations, not values to subtract.

Addition receives actual `Option<Box<Node>>` chains built before timing. Returned nodes are walked to capture values and released iteratively inside each evaluation. Retained input chains are released iteratively after timing, including ordinary error paths. Safe ownership prevents returning borrowed input nodes as owned output through the supplied signature. The driver does not impose a custom solver representation beyond that public interface.

The C and Rust scaffolds have disclosed setup differences: C retains parsed arrays and builds contiguous linked input nodes; Rust drops those temporary arrays after constructing individual boxed input nodes. Their memory footprints and locality therefore differ. Rust and C also use their native allocation, output-normalization and safety mechanisms. The measurement is solve/capture/hash/cleanup, not isolated algorithm time; whole-child peak RSS includes preparation.

## Validation performed

On this Apple Silicon host, `rustc 1.90.0 (1159e78c4 2025-09-14)` compiled all three configurations under the strict flags above. Tests linked separately compiled, temporary constant or identity fixtures outside the repository. These fixtures are generic harness probes, not trial solutions.

The preparation checks passed **160 assertions**, covering:

- Standalone output and accepted ASCII whitespace for all three interfaces.
- Equivalent C and Rust fixtures across 40-case pools, seven rounds and three warmups, in both evaluate and control modes. Every report field except elapsed time matched, including an independently computed wrapping checksum. Forty cases exercise the positional weight's wrap at 17.
- Full preflight rejection in both modes, incorrect expected lengths, incomplete or trailing pool data, zero/oversized pools, and successful 256-case pools.
- A stateful non-solver fixture confirms that changed results fail the warmup or timed checksum, and that control calls the solver only during preflight.
- Invalid CLI modes/counts/arity, overflowing arguments, zero rounds, excessive calls/warmups, plus signs, missing/suffixed tokens, negative lengths, collection limits and signed integer overflow.
- Exact capture and wrapping hashes for negative values and both signed 64-bit endpoints; empty list output; repeated capture, hashing and iterative cleanup of 10,000-node lists.
- Accepted 10,002-node output and rejected 10,003-node output, including cleanup of the rejected chain.
- Rejection of NaN, infinity, quarter-integer and out-of-range median fixtures; acceptance of negative half-integers, negative zero and the lower doubled-i64 endpoint.

The fixed short benchmark calls above validated the timing/report path; their elapsed values were not used for performance conclusions. The coordinator should freeze the retained harness and corpus hashes, integrate its reusable grading self-tests, and complete the independent experiment checks before starting measured workers. This handoff does not mark the broader experiment complete.
