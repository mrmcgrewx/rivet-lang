# C authoring-experiment harness handoff

Unmeasured setup work completed 2026-10-25 against the harness contract committed in `5b37659` and `AGENT-EXPERIMENT-PLAN.md`. This contribution owns only `experiments/agent-language-v1/harness/driver.c`, `c_backend.c`, and this document. It contains no Two Sum, list-addition or median solver. No worker submission, production code, shared header or Git history was changed.

## Interfaces and ownership

The driver uses the committed `backend.h` unchanged. Compile with `TASK=1`, `TASK=2` or `TASK=3`. The C backend uses `api.h` unchanged and calls the separately compiled worker's `solve` with arrays, actual singly linked input nodes, or arrays respectively. The same driver can link the coordinator's Rivet backend instead.

- The driver owns parsed `Case.a`, `Case.b`, expected vectors and the case pool. The backend owns prepared `Case.state` and releases it in `backend_finish` after timing.
- For C list inputs, the backend creates ordinary `Node` records in a contiguous allocation for each chain and links their `next` pointers. The worker receives actual node pointers, not an array adapter. These input allocations survive all repetitions and are released after measurement. Parsed input vectors also remain owned by the common driver, so peak memory includes both representations and must not be called auxiliary search memory.
- Returned list nodes must each be individually heap allocated and distinct from input nodes, as required by the worker contract. The backend captures values into the fixed `Answer.values` buffer, then frees every valid result node individually and iteratively in `backend_release` inside the timed loop. No recursive destruction is used.
- Capture is capped at 10002 values. Oversized or cyclic chains fail before any node is freed, avoiding double-free or an unbounded traversal. Current-input node aliases are rejected before dereference/free. Invalid-result processes exit and their remaining allocations are reclaimed by the operating system. Arbitrary invalid pointers, stack-returned nodes, or allocator-contract violations are C solver undefined behavior; the scaffold cannot make arbitrary C pointers memory-safe. Coordinator sanitizer runs and process failures remain part of judging these cases.
- `Answer` is reused without clearing its approximately 80 KiB value buffer per query. `backend_solve` must initialize `n`, `scalar`, `owned`, `ticks` and the returned value range; the C adapter does so. C ticks are always zero. The coordinator's Rivet adapter must follow the same metadata contract.

## Driver behavior

Default mode reads exactly one task input from stdin, prepares input state, invokes the solver, normalizes the captured answer, prints the integer array or finite scalar, releases the result and finishes the backend. Scalars use `%.16e` plus newline. The parser streams ASCII whitespace-separated signed i64 tokens, permits leading zeros/negative zero, rejects plus signs/invalid suffixes/overflow and requires exact EOF. Each input array is bounded to 10000 elements; the whole stream has a 128 MiB ceiling, above the largest permitted 256-case pool.

Benchmark mode is exactly:

```sh
program --bench POOL ROUNDS WARMUPS evaluate
program --bench POOL ROUNDS WARMUPS control
```

It loads 1..256 cases, each followed by the supplied expected vector. Rounds must be positive, total timed calls cannot exceed 100000000, and warmup rounds are bounded to 10000. Argument numbers must be decimal digits without signs. All cases are fully evaluated and compared with the supplied oracle vectors in preflight, even in control mode. This is comparison scaffolding; it does not compute the oracle or solve a task.

Pair indices and expected pair order are normalized. Scalar answers are required to have a finite, integral signed-i64 doubled value, then represented as the one-value vector `[2 * median]` for comparison/hash purposes. Valid experiment inputs guarantee that representation. List values are compared exactly without special decimal-digit arithmetic in the harness.

The checksum follows the committed contract exactly: start at uint64 `1469598103934665603`, fold length then each normalized signed value using XOR and multiplication by `1099511628211`, with unsigned wrapping. Per-round hashes are weighted by `1 + index % 17` and summed with wrapping. The final checksum must equal the wrapping product of the preflight expected sum and rounds. Signed values convert to uint64 modulo 2^64 before folding.

Warmups use the selected mode. Timed evaluate calls include solve, fixed-buffer output capture, normalization, hashing and per-result release. Parsing, preparation, exact vector preflight comparisons, printing and backend finish are outside the monotonic interval. Control reads the expected hash through a volatile access and includes the same outer weighting/sum loop; it is a floor and is never subtracted. JSON fields match the fixed contract, including exact unsigned integer checksums and preflight `ticks_per_case`. Very short control probes can report zero nanoseconds at clock resolution; production calibration must not infer infinite speed or subtract such measurements.

## Validation actually performed

Built all three `TASK` variants with separate temporary stub-solver translation units using:

```sh
clang -std=c11 -O3 -Wall -Wextra -Wpedantic -Wconversion -Wshadow -Werror \
  -DTASK=1 -I experiments/agent-language-v1/harness \
  experiments/agent-language-v1/harness/driver.c \
  experiments/agent-language-v1/harness/c_backend.c /tmp/STUB.c -o /tmp/PROGRAM
```

The task-3 build initially exposed an unused local error helper; its compilation is now restricted to the task variants that use it. All variants then passed strict compilation. The checks were repeated with `-O1 -g -fsanitize=undefined -fno-sanitize-recover=all`. No ASan success is claimed by this handoff; coordinator holdout sanitizer acceptance remains separate.

An external temporary Python checker performed **147 child checks**, all passing in the final run. Fixtures were a constant reversed pair, an input-list clone, and a first-value scalar echo; these are generic harness fixtures, not problem solutions. They were created outside the repository and their temporary executable/source directory was removed after the run.

Coverage included exact standalone output, signed i64 extrema, normalized pair order, full list values, fractional/negative scalar output, separately computed Python wrapping checksums, expected hash/call/warmup/tick fields, evaluate/control modes, preflight failure even in control mode, and a stateful stub changing its result after preflight to exercise the final checksum failure. Malformed token, extra input, excessive/negative length, invalid/overflowing/zero call counts, excessive warmups and unknown mode all failed without stdout. Additional stub probes confirmed nil/capture-boundary list output, 10003-node rejection, bounded cyclic-list rejection, input-alias rejection, and rejection of NaN/infinity/non-half/overflowing scalar captures. All result destruction for valid chains was iterative; the 10002-node boundary case passed under UBSan.

`git diff --check` passed for the two implementation files. No measured authoring trial or runtime comparison was run by this helper. The coordinator must integrate the Rivet backend, freeze the complete wrappers/oracles/corpora and hashes before starting workers, verify this driver's checksums against Rust/Rivet, run C holdout sanitizers, and report scaffold overhead/data-structure differences in the final pilot results.
