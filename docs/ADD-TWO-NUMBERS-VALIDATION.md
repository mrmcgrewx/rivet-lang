# Add Two Numbers / linked-data validation

Validated on 2026-10-25 with macOS 26.5.2 arm64, Rust 1.90.0, Apple Clang 17.0.0 (`clang-1700.4.4.1`), Node 22.7.0 and Playwright 1.64.1. The design/ownership/roadmap checkpoint is `8a6b220`; implementation is `432cd26b4c4cf4a848d8b41695a54f681c4890c4`. This record extends the prior `TWO-SUM-VALIDATION.md` without replacing its historical evidence.

## Implemented result

`examples/add-two-numbers.rvb` contains 657 actual source bytes, 70 numeric instructions and 17 typed registers. Its input traversal, validation, digit sum, carry, node construction and reversal compile to native operations and branches. `IntList` handles refer to immutable linked nodes; this is not an array-only algorithm or a runtime solver. The runtime only supplies general node storage/input/output and existing bounds.

One executable accepts two count-prefixed reverse-order digit lists. The three published examples return `[7,0,8]`, `[0]`, and `[8,9,9,9,0,0,0,1]`. A 100-digit all-nine number plus one returns 100 zero digits followed by one, without converting the entire number to i64. Program assertions reject invalid lengths/digits/noncanonical leading zeros; native assertion diagnostics identify the source PC.

The release-built local deliverable `.rivet/add-two-numbers-deliverables/add-two-numbers` is 35544 bytes. `file` reports arm64 Mach-O and `otool -L` lists only `/usr/lib/libSystem.B.dylib`. Its metadata reports compute ABI 2. Three launches of this same executable passed the published inputs.

## Working-checkout verification

- Formatting, strict all-target Clippy, release build and the existing full suite with the new algorithm/API tests passed (76 tests). The subsequently added nine generic linked-list tests also passed; the clean-clone gate below passed all 85 together.
- `tests/add_two_numbers.rs` compares 270 valid pairs (14 published/edge/boundary cases plus 256 seeded randomized pairs) against Python's arbitrary-precision integer addition as an independent test oracle. Every case also matches reference execution and both native profiles: 540 native valid-case runs. Additional malformed-input cases, source assertions, CLI input, 101-digit results and executable-only copies pass. Python is test-only, absent from native execution.
- Nine generic compiler/list tests cover every copied type, map aliases, shared immutable tails, nil semantics, in-place updates, source byte/type/CFG errors, signed div/rem guards, assertion PC diagnostics, logical quota order and 4000 mutation probes. Twenty generic native artifacts across both profiles and 58 launches match reference outcomes. A 10000-node echo preserves 210002 output bytes; reference cleanup succeeds on a 128 KiB thread stack.
- Strict C, baseline/size and UBSan suites pass, including previous text/JSON/HTTP/array/map tests and new list helper tests for shared tails, deep chains, exact input/output, nil/type errors and combined quota boundaries. See the runtime handoff for detailed cases. No ASan result is claimed.
- Six workbench/API tests pass. The new case checks the exact fifth binary example, compute inspection, one build reused for all three inputs, assertion failure and successful recovery without rebuilding.
- All 16 real Chromium scenarios passed in 24.7 seconds, including the three examples, 100-digit carry, unchanged source/export/build ID across input edits, malformed digit recovery and existing examples/layouts.

Compiler, runtime and UI implementation handoffs preserve individual validation details. The completed independent review in `ADD-TWO-NUMBERS-REVIEW.md` found no unresolved blocker. Its separate path-state oracle agreed on 10000 generated CFGs; 50000 mutations plus 657 truncated prefixes completed without panic, with 2979 accepted mutation cases preserving bytes. A separate signed-list transformer passed 72 native differential runs and four UBSan probes, beyond the addition algorithm.

## Actual in-app browser

The coordinator refreshed the release workbench on port 7331 and opened Add Two Numbers. The GUI showed `IntList` instructions and 657 binary bytes. Build produced a native artifact, and all three published cases ran using build #1. Invalid digit 10 produced `assertion failed at instruction @33`; the same build then handled the 100-digit carry case successfully. Returning to the default input produced `[7,0,8]`; the hex view and output were captured together.

- [Binary source and native output](evidence/add-two-numbers.png)
- [Decoded linked-list operations and carry example](evidence/add-two-numbers-carry.png)

The workbench remains available at `http://127.0.0.1:7331/`, with the new example selected in the verification tab. Source edits and runtime input remain separate. Byte-exact import/export and no-rebuild behavior are also covered by browser regression tests.

## Capability and performance scope

These are reusable language additions: nullable immutable lists, handle copying/aliasing, guarded signed division/remainder and assertions. The linked-list runtime does not contain decimal-digit checks or addition logic. The algorithm is O(max(m,n)) in time with linear allocated node storage, including a temporary reversed chain. This is algorithm analysis, not a comparative runtime benchmark. The roadmap explicitly tracks functions/modules/libraries, practical agent tools and fair throughput/memory/authoring measurements before claiming parity or superiority to established languages.

Existing RVB1/RVB2 examples remain byte-exact and retain their semantics. Lists remain immutable i64-only nodes with a 10000-node chain bound; there are no user-defined records, functions or garbage collection yet. Older compilers reject the newly added opcodes. Verification targets Apple Silicon, and no LeetCode account submission is claimed.

## Clean reproduction and public CI

A fresh public HTTPS clone of `432cd26b4c4cf4a848d8b41695a54f681c4890c4` at `/tmp/rivet-add-numbers-clean.Ok0Rff` passed these checks with fresh Cargo build outputs and npm installation:

```sh
python3 scripts/write_binary_examples.py
git diff --exit-code -- examples/
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
python3 tests/native_runtime/run.py
python3 tests/native_runtime/run.py --sanitize undefined
npm ci
npx playwright install chromium
npm test
```

All **85 Rust tests** passed together: 2 runtime, 2 addition/CLI, 13 RVB1 binary, 10 compute, 3 transport, 24 legacy semantics, 9 linked-data, 5 native artifact/lifecycle, 8 native codegen, 3 Two Sum/process/CLI and 6 API tests. All **16 browser scenarios** passed in 29.0 seconds. Strict C baseline/size and UBSan suites passed. All five source fixtures regenerated byte-for-byte, and `git status --short` stayed empty.

The clean release compiler produced the same-sized 35544-byte native artifact. A copy placed alone in a fresh temporary directory passed the three published cases and the 100-digit carry case with an empty environment; metadata reported ABI 2. No Rivet/compiler/source/sidecars were present in that directory. Temporary standalone files were removed.

[Public CI run 40230249273](https://github.com/mrmcgrewx/rivet-lang/actions/runs/40230249273) succeeded for implementation `432cd26b4c4cf4a848d8b41695a54f681c4890c4` on 2026-10-25. Formatting, compiler warnings, exact binary fixture regeneration, compiler/runtime/API tests, native support and undefined-behavior checks, and browser acceptance all passed.
