# Median benchmark implementation handoff

Completed benchmark machinery 2026-10-25 against `MEDIAN-PLAN.md`, committed before delegation in `cccf624`. Files are confined to `benchmarks/median/`, `scripts/median_benchmark.py`, `MEDIAN-BENCHMARK-METHOD.md`, and this handoff. No production compiler, runtime, source example, UI, shared state or Git changes were made by the benchmark owner. The coordinator owns final measurements and conclusions after a committed, quiet validation checkpoint.

## Deliverables

- `benchmarks/median/partition.c/.h`: ordinary C implementation of the same shorter-array binary partition, compiled as a separate translation unit with no LTO.
- `benchmarks/median/standalone.c`: standalone valid-input C frontend and canonical float output.
- `benchmarks/median/median.py`: matching Python algorithm, standalone entry and preloaded entry. Benchmark-only imports are lazy so they do not inflate standalone startup.
- `benchmarks/median/preloaded.c`: shared C/Rivet preloaded harness and unsubtracted control floor. The Rivet mode includes the real production runtime, supplies actual prebuilt array handles, resets fuel per independent call, captures the float result, checks boundary counts and unchanged allocation charge, and performs real cleanup after measurement.
- `scripts/median_benchmark.py`: reproducible corpus/oracle validation, speed builds, narrowly reversible LLVM symbol substitutions, independent per-child `wait4` RSS, process and preloaded timing scopes, warmups, calibration, randomized sample orders, weighted checksums, raw report and summary distributions.
- `MEDIAN-BENCHMARK-METHOD.md`: exact reproduction commands, substitution contract, optimizer visibility, safety/parser differences, timing/memory units and limits on interpretation.

No tick, arithmetic guard, branch, bounds/type check or generated algorithm operation is removed for the harness. Only main/input/output/cleanup boundary symbols change in a separate copy of emitted LLVM. Reversing those substitutions must recover the original bytes exactly. For the current program the smoke verified 81 unchanged static tick call sites; generated reads/returns/cleanup counts matched their runtime contract.

## Validation actually performed

The following passed:

```sh
clang -std=c11 -O3 -Wall -Wextra -Werror -Wpedantic -Wconversion -Wshadow \
  benchmarks/median/partition.c benchmarks/median/standalone.c -o /tmp/median-c-review
clang -std=c11 -O3 -Wall -Wextra -Werror -Wpedantic -Wconversion -Wshadow \
  benchmarks/median/partition.c benchmarks/median/preloaded.c -o /tmp/median-c-preloaded-review
python3 -m py_compile benchmarks/median/median.py scripts/median_benchmark.py
cargo build --release --locked
python3 scripts/median_benchmark.py --samples 1 --cases-per-group 2 --target-ms 1 \
  --report .rivet/median-benchmark-smoke.json
git diff --check -- benchmarks/median scripts/median_benchmark.py docs/MEDIAN-BENCHMARK-METHOD.md
```

The complete smoke was rerun after adding published tiny cases and moving Python's benchmark-only imports. Its final result is explicitly marked `functional_smoke`, not a performance comparison:

- Sixteen inputs across all eight families agreed with the independent sorted-union oracle through all three unchanged standalone executables: 48 standalone output checks.
- All preloaded children individually verified every case, then their repeated weighted checksums and call counts matched independent expectations.
- 120 raw observations were recorded across process warmup/measurement, preloaded calibration/measurement, and control samples.
- The native transformed harness built with strict C warnings and real runtime checks retained. Executed fuel counts ranged from 57 to 290 across these smoke cases.
- Each child produced a separate RSS measurement with raw and normalized values. The local macOS `getrusage(2)` man page was checked and specifies `ru_maxrss` in bytes; the script multiplies Linux KiB values by 1024. Linux execution was not tested.

The smoke report and its builds/datasets are ignored local artifacts under `.rivet/`; numerical smoke timings must not be presented as performance findings. The report captures the exact toolchain/interpreter/host versions and hashes used. AddressSanitizer/UBSan runs here were not needed to claim benchmark correctness; production runtime sanitizer evidence belongs to the coordinator's existing suite.

## Next acceptance step

After committing the implementation and finishing clean-checkout validation, run the default seven-sample comparison while build/test jobs are idle:

```sh
python3 scripts/median_benchmark.py --report .rivet/median-benchmark.json
```

Preserve the raw final report alongside any published analysis, including its revision/source/runtime/compiler hashes and working-tree state. The coordinator should report both ordinary process and preloaded scopes, show slower as well as faster outcomes, and distinguish one-case process RSS from the whole preloaded family RSS. Do not subtract the control floor or startup estimate, treat RSS as auxiliary algorithm memory, or generalize this one workload into a language-wide claim. Independent review by the UI/review owner was requested before final acceptance.
