# Two Sum / RVB2 validation

Validated on 2026-10-25, macOS 26.5.2 arm64, Rust 1.90.0 and Apple Clang 17.0.0 (`clang-1700.4.4.1`). Target: `arm64-apple-darwin25.5.0`. Node 22.7.0 / Playwright 1.64.1 provide browser checks. The committed contract is `fe95cd0`; this record concerns the new computation milestone, retaining earlier RVB1 evidence in `BINARY-VALIDATION.md`.

## Result

`examples/two-sum.rvb` is an actual 231-byte binary program containing 21 operations and 13 typed registers. It compiles to a standalone arm64 Mach-O executable. The native program reads an array and target from stdin and returns indices. Expected linear-time scanning is implemented in its own numeric records and generated native branches; linked C supplies generic arrays, maps, input/output and bounds. No algorithm-specific solver, JSON source, source decoder or reference interpreter is in the executable.

The size-profile deliverable `.rivet/two-sum-deliverables/two-sum` was built locally: 35384 bytes. `file` identified arm64 Mach-O; `otool -L` listed only `/usr/lib/libSystem.B.dylib`. Three separate launches of that same file produced:

| Runtime input | Output |
| --- | --- |
| `4 2 7 11 15 9` | `[0,1]` |
| `3 3 2 4 6` | `[1,2]` |
| `2 3 3 6` | `[0,1]` |

The source does not change across inputs. Tests also copy only the executable into an otherwise empty directory and launch it with an empty environment, independently of Rivet/workbench/source/sidecars. Build metadata identifies the compute support ABI as version 2.

## Checks performed in the working checkout

- `cargo fmt --check`, strict all-target Clippy and release build passed.
- The full Rust suite passed 72 tests; a subsequently added process-regression test passed with the three-test Two Sum suite, bringing current coverage to 73 tests. The final clean run below is the combined acceptance gate.
- Two Sum native/reference tests use both baseline and size profiles. Each artifact handles 107 small cases (including all three published examples and 96 seeded unique-answer cases checked by a separate brute-force oracle), a 10000-element case with over 64 KiB stdin, malformed input and independent copying. CLI `run` and `interpret` use the same binary source and separate stdin data.
- Ten generic compute tests include type/control-flow/definite-assignment failures, exact input/bounds behavior, loop and allocation failures, 8000 mutation/roundtrip probes, and 18 native differential builds across nine generic programs and both profiles. The prior RVB1 tests still pass.
- Process regressions independently compile a generic array echo that returns 210002 bytes and a self-jump program that exhausts fuel without reading a full 262144-byte stdin pipe. Both native profiles preserve complete output and terminate/reap without deadlock.
- Five workbench/API tests include binary example delivery, one build reused across multiple inputs, runtime failure recovery, decoded input bounds, and an exact 256 KiB valid input whose vertical-tab whitespace expands sixfold in the JSON control envelope.
- `python3 tests/native_runtime/run.py` and `--sanitize undefined` passed, including strict C warnings, baseline/size generic compute tests and all previous value/HTTP cases. `TWO-SUM-RUNTIME-HANDOFF.md` records the cases. No ASan success is claimed.
- All 14 real Chromium scenarios passed. New cases preserve binary source and build IDs across input changes, preserve integers beyond JavaScript's exact Number range, handle runtime errors/input bounds, and retain compact layouts.

## Actual in-app browser acceptance

The coordinator refreshed the workbench at `http://127.0.0.1:7331/`, opened Two Sum, inspected RVB2 decoded operations and binary hex, built a native artifact, and ran all three published examples. Changing runtime input returned the expected new indices under the same build ID and executable path. The final compiler's compact decoded register list exposes the operations promptly. Screenshots were captured from the actual in-app browser:

- [Binary bytes and native result](evidence/two-sum-hex.png)
- [Decoded source and native result](evidence/two-sum-decoded.png)

The browser suite separately verifies byte-exact download/import and no rebuild for input-only edits. Existing Hello/HTTP functionality remains covered by the full regression suite.

## Review and limits

`TWO-SUM-REVIEW.md` records an independent review of compiler/algorithm/integration by the runtime implementer. Its separate CFG oracle agreed on 10000 generated graphs; 59367 mutation/truncation cases completed, with 6534 accepted variants preserving exact bytes. A metadata ABI mismatch was corrected and regression-tested. No unresolved correctness blocker was reported. This is substantial bounded evidence, not exhaustive verification.

RVB2 is one computation with integer/boolean registers, immutable integer arrays and mutable integer maps; it is not yet a general application platform. Fuel, input/output, array/map and logical allocation limits remain explicit. The map reserves a fixed capacity and does not guarantee worst-case constant-time lookup. Full-i64 input beyond LeetCode's domain can fail checked complement arithmetic. No LeetCode account submission was made; acceptance here is local native/reference/oracle execution. No AI authoring efficiency or cross-platform claim is made.

## Clean reproduction

Pending at this implementation checkpoint. A fresh clone and final combined checks must be recorded before marking the milestone complete.
