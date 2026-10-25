# Same-model agent authoring: Rivet, C and Rust

Nine fresh agents independently solved Two Sum, Add Two Numbers and Median of Two Sorted Arrays. Every agent used **gpt-6-astra with xhigh reasoning**, verified from its session metadata. Every solution passed its first formal submission and all withheld cases. C led the large Two Sum and linked-list addition workloads; Rust led balanced median search and used the fewest output tokens. Rivet produced correct native programs directly from newly authored binary source, but this pilot does not demonstrate an authoring advantage.

This is **one independently authored solution per language/problem pair**, on one machine. It compares the programs the agents chose to write, including their libraries and algorithm variants. It does not isolate a universal language effect or estimate a reliable success rate for unseen tasks.

## Frozen method and correctness

The [protocol](AGENT-EXPERIMENT-PLAN.md) was committed at `b5066c8`; [packets, corpus and harness hashes](../experiments/agent-language-v1/manifest.json) were frozen at `7461e96` before any measured agent started. Source snapshots and raw results were published at `eb39fdc`. No production compiler/runtime or prior example was changed for this experiment.

Workers had fresh context, the same problem definitions, a maximum of three formal submissions, and only their assigned packet. C/Rust received generic solver interfaces; Rivet received its binary-format specification and could write an encoding-only Python tool. No worker accessed previous implementations, hidden cases, the grader implementation or the network. Recorded tool commands and source/packet hashes passed the [protocol audit](../experiments/agent-language-v1/results/protocol-audit.json). Isolation was procedural and audited, not an OS security boundary. Coordinator, harness-preparation and review work is excluded from worker usage.

All nine first submissions passed **10 development cases each**. Each language then passed **256 Two Sum, 264 list-addition and 358 median holdouts: 878/878 per language, 2,634 native checks overall**. There were **zero retries**. These easy-to-author, familiar tasks produced no evidence for a reliability difference. No repairs were allowed after holdout feedback.

Independent oracles use exhaustive pair search for small Two Sum inputs and value-multiplicity counting for larger inputs, Python arbitrary-precision addition for lists, and sorted-union medians. Corpora include empty median arrays, negatives, duplicates, carry chains, maximum lengths, exhaustive small sorted arrays and seeded random inputs. All Rivet holdouts also matched the reference interpreter. All C holdouts passed local UBSan and [Linux CI ASan/UBSan](https://github.com/mrmcgrewx/rivet-lang/actions/runs/40239461018). Apple's local ASan hung before `main`, including for an empty executable; the [infrastructure record](AGENT-EXPERIMENT-INFRASTRUCTURE.md) explains the separate CI check. Sanitized runs are excluded from timing.

## Actual authoring token usage

These are recorded session counters, not agent estimates. Input is cumulative across model calls and **already includes cached input**. Output **already includes reasoning output**. Uncached input equals input minus cached input. Common instructions, packet reads, source construction, tool feedback and handoffs are included. Cache residency differs between sessions, so uncached totals do not establish intrinsic language efficiency or a dollar cost. Rivet's binary specification and byte-writing work count toward its usage.

Totals across three problems:

| Language | Input, including cache | Cached input | Uncached input | Output | Reasoning subset of output |
| --- | ---: | ---: | ---: | ---: | ---: |
| Rivet | 726,817 | 693,248 | 33,569 | 9,688 | 2,994 |
| C | 682,653 | 667,008 | 15,645 | 5,810 | 1,947 |
| Rust | 603,038 | 588,288 | 14,750 | 4,086 | 1,199 |

Output totals were 9,688 for Rivet, 5,810 for C and 4,086 for Rust. They include the retained byte writers and handoffs, not just the final binary size. The raw [usage report](../experiments/agent-language-v1/results/usage.json) preserves every cumulative numeric observation and each verified model/effort configuration, without publishing private session transcripts or reasoning text.

| Problem | Language | Uncached input | Output | Formal submissions | Holdouts |
| --- | --- | ---: | ---: | ---: | ---: |
| Two Sum | Rivet | 17,986 | 2,710 | 1 | 256/256 |
| Two Sum | C | 5,254 | 2,028 | 1 | 256/256 |
| Two Sum | Rust | 5,017 | 1,103 | 1 | 256/256 |
| Add Two Numbers | Rivet | 7,898 | 3,117 | 1 | 264/264 |
| Add Two Numbers | C | 5,074 | 1,312 | 1 | 264/264 |
| Add Two Numbers | Rust | 4,872 | 1,222 | 1 | 264/264 |
| Median | Rivet | 7,685 | 3,861 | 1 | 358/358 |
| Median | C | 5,317 | 2,470 | 1 | 358/358 |
| Median | Rust | 4,861 | 1,761 | 1 | 358/358 |

## Runtime

Apple M1 Pro, macOS 26.5.2 arm64; Apple Clang 17.0.0 and Rust 1.90.0. C uses `-O3` with separate solver/driver translation units and no LTO. Rust uses a separate solver rlib, `opt-level=3`, checked integer overflow, aborting panics, no LTO and safe code only. Rivet uses its speed profile with all arithmetic, bounds, fuel and allocation checks. The languages do not provide identical safety guarantees.

The primary measurement preloads eight inputs per family, then times **solve, output capture/normalization, checksum and result cleanup**. Parsing, input construction, preflight, warmup and printing are outside that clock. Rivet substitutes only read/return/cleanup boundaries in generated LLVM, reversibly; every program operation and fuel tick remains. Native and preloaded outputs were cross-checked against the same expected values. Inputs are real runtime arrays/lists; query allocations use real cleanup. C/Rust also release result nodes inside the timer.

Seven samples per language/family, deterministic shuffled order, 32 warmup rounds, calibrated repetitions targeting 50 ms with a two-million-call cap. All authoring agents had stopped; no tests or profiling ran during measurement. This was an ordinary desktop host, not a dedicated isolated benchmark machine. Raw min/median/max, every sample order, checksums, fuel counts, calibration and per-child CPU/RSS are retained. Control-loop medians were 0.63–1.36 ns/call and are **not subtracted**. The fastest cases can hit the call cap before 50 ms; raw calibration records make that visible.

Median preloaded nanoseconds per call (lower is faster):

| Problem | Input family | Rivet | C | Rust |
| --- | --- | ---: | ---: | ---: |
| Two Sum | small_8 | 7,025.82 | 40.30 | 106.24 |
| Two Sum | medium_1000 | 9,798.59 | 1,816.43 | 8,811.01 |
| Two Sum | large_10000 | 68,548.91 | 34,059.03 | 92,473.88 |
| Add Two Numbers | small_1 | 96.16 | 41.57 | 46.84 |
| Add Two Numbers | medium_32 | 1,807.54 | 732.13 | 792.69 |
| Add Two Numbers | large_100 | 5,527.32 | 2,273.53 | 2,484.63 |
| Median | small_4 | 30.50 | 6.87 | 7.85 |
| Median | medium_100 | 67.99 | 14.57 | 13.51 |
| Median | large_1000 | 100.49 | 20.81 | 17.46 |
| Median | asymmetric | 33.33 | 4.78 | 7.25 |

For large inputs, Rivet took 2.01× C's time on Two Sum, 2.43× on list addition and 4.83× on balanced median search. Rust's large Two Sum took 1.35× Rivet's time, while Rust's balanced median took 0.84× C's time. These are observed program results, not matched-implementation language ratios. The prior hand-authored median benchmark uses different programs and inputs and is not a before/after baseline for this table.

The source audit explains relevant differences:

- **Two Sum:** all agents chose complement lookup before insertion. C uses a capacity-scaled open-addressed table, Rivet's runtime creates a fixed 32,768-slot table, and Rust uses `std::collections::HashMap`. Rivet's tiny-input result (7,025.82 ns versus C's 40.30 ns) is consistent with substantial fixed map setup cost; this pilot did not profile the exact fraction. Expected linear search is not a worst-case linear guarantee.
- **List addition:** C/Rust append one result chain. Rivet's immutable nodes require a temporary reversed chain followed by a second constructed chain, adding allocations and traversal. All are linear, but their work and auxiliary storage differ.
- **Median:** C chooses a lower-bound partition search that loads fewer values per iteration. Rust uses guarded bidirectional partition search; Rivet clamps the valid partition range and examines four boundary values. All have logarithmic search and constant auxiliary search storage.

## Memory and process latency

`wait4` records each child's own peak resident set. RSS includes the executable/runtime, parser, all eight preloaded inputs, preflight, warmup and allocations. It is **not algorithm-only auxiliary memory**, and it is not obtained by subtracting a previous child peak. List input layout differs: C uses contiguous linked nodes while retaining parsed arrays; Rust uses individual boxes and drops those arrays; Rivet retains runtime nodes plus driver arrays. Output capture is reused. These setup differences affect RSS and locality.

Median preloaded whole-child peak RSS for the large families, MiB (1 MiB = 1,048,576 bytes):

| Problem | Rivet | C | Rust |
| --- | ---: | ---: | ---: |
| Two Sum | 3.438 | 2.484 | 2.625 |
| Add Two Numbers | 1.547 | 1.438 | 1.688 |
| Median | 1.641 | 1.484 | 1.812 |

A separate scope launches a fresh standalone process for one representative input per family, including parsing and output. These startup-dominated times do not measure solver speed; different parsers and runtime initialization affect them. Seven-sample large-family medians:

| Problem | Rivet process ms / peak MiB | C process ms / peak MiB | Rust process ms / peak MiB |
| --- | ---: | ---: | ---: |
| Two Sum | 2.308 / 2.328 | 5.718 / 1.938 | 2.994 / 2.016 |
| Add Two Numbers | 1.790 / 1.438 | 1.973 / 1.359 | 2.027 / 1.562 |
| Median | 1.897 / 1.406 | 2.343 / 1.375 | 2.228 / 1.625 |

## Retained programs and reproduction

Every original submission, byte writer, handoff and grading record is retained under [results/trials](../experiments/agent-language-v1/results/trials). There were no unsuccessful submissions to omit. Rivet's source files are 231, 340 and 687 bytes (21, 35 and 67 operations), independently authored for this pilot. Their speed-profile executables are 34,952, 34,920 and 34,808 bytes respectively. These source sizes exclude the authoring tools and should not be confused with model token cost.

Rebuild and recheck on the verified Apple Silicon target from a clean checkout:

```sh
cargo build --release --locked
python3 experiments/agent-language-v1/test_harness.py -v
python3 experiments/agent-language-v1/evaluate.py   --sanitizer-mode undefined --output .rivet/reproduced-correctness.json
python3 experiments/agent-language-v1/measure.py   --correctness .rivet/reproduced-correctness.json   --verify-only --output .rivet/reproduced-boundaries.json
```

To repeat runtime/RSS measurements, stop other builds/tests/agents and omit `--verify-only`, choosing a new output file. Evaluation builds go into a fresh temporary directory by default; retained original JSON may name temporary paths from the original host, so rebuild rather than relying on those paths. On a working ASan host, the default evaluator adds ASan; the portable C-only check is:

```sh
python3 experiments/agent-language-v1/sanitizer_check.py   --output .rivet/c-sanitizers.json
```

Reproduction executes the retained solutions. Repeating agent authorship is a new experiment: use fresh workspaces, new sessions and a separately frozen protocol. The archived packets contain original registration paths; `prepare.py` is the original one-time registration script, not a command that resets this measured run. Actual token counters require the original private sessions; the published numeric observations preserve the evidence without exposing their contents.

The new median binary was opened through **Open binary** in the actual workbench, decoded as 67 operations, built into a standalone executable, and returned 2.5 for `[1,2]` and `[3,4]`. The UI uses its normal size profile; this is functional proof, not a benchmark timing sample.

![Agent-authored median binary decoded and run natively](evidence/agent-pilot-median.png)

## What this establishes

The same model can independently author small, correct Rivet binary programs from the format specification, with no access to existing solutions. It did so on the first submission for all three tasks. This run provides no correctness or retry advantage over C/Rust; Rust used less generated output, C was faster in two large workloads, and Rust was faster in the third. Rivet's compact binary representation by itself did not reduce agent authoring effort here.

Three familiar LeetCode problems, one agent per cell, a single model/effort and one desktop cannot establish the best language for LLMs or production workloads. Training familiarity, incidental cache state, documentation/byte-writer overhead, independently chosen algorithms, native libraries and unequal safety contracts remain confounders. A stronger follow-up would use multiple fresh agents per cell and unfamiliar problem variants, with a second matched-algorithm track to separate compiler/runtime costs from author choices. The fixed-size map and immutable-list allocation pattern are concrete Rivet improvement candidates; changing them is outside this frozen experiment.

Raw evidence: [correctness](../experiments/agent-language-v1/results/correctness.json), [usage](../experiments/agent-language-v1/results/usage.json), [690 runtime/RSS observations](../experiments/agent-language-v1/results/performance.json), [CI sanitizers](../experiments/agent-language-v1/results/ci-sanitizers.json), [protocol audit](../experiments/agent-language-v1/results/protocol-audit.json). See the [independent review](AGENT-EXPERIMENT-REVIEW.md) and [validation record](AGENT-EXPERIMENT-VALIDATION.md) for final acceptance evidence.
