# Independent native implementation review

Reviewed 2026-10-25 under the ownership committed in `279dd92`, against the native completion contract in [NATIVE-PLAN.md](NATIVE-PLAN.md). The reviewer owns this file only and made no production or Git changes. The reviewed snapshot includes the native core at `279dd92` and the UI owner's uncommitted native UI updates.

## Scope

Read the project instructions, brief, architecture, state, native plan, implementation contract, prior validation/review records, and native compiler/runtime handoffs. Inspected the direct LLVM lowering, native C ABI and allocation/HTTP implementation, Clang builder, OS-child lifecycle, CLI/workbench integration, artifact downloads, UI source, and their relevant tests.

The native implementation meets the architectural correction: the compiler emits direct LLVM operations and direct calls into C support, and Clang produces linked executables. Checked integer addition is an LLVM overflow intrinsic and branch. The executable does not parse source, dispatch bytecode, call the Rust interpreter, or depend on the workbench. Both console and HTTP artifacts use the same builder and execution path. Program values determine console output and registered HTTP routes/status/body. Typed graph representation, failure ordering, helper boundary, immutable values, and the concrete size-versus-baseline objective are specified in the native plan without unsupported model-reliability or performance claims.

## Finding and resolution

### P2: SIGTERM of the workbench leaves the native server running

**Initial locations:** `Cargo.toml:21` and `src/main.rs:53-55`.

The CLI registers `ctrlc::set_handler`, but `ctrlc` is enabled without its `termination` feature. On this Unix target the handler therefore handles SIGINT, while SIGTERM terminates the workbench directly. The owned `NativeServer` destructor and `Workbench::serve` shutdown path do not run, leaving the native server child and listening port alive. This contradicts the planned shutdown behavior and is a regression from running the reference server inside the parent process.

**Independent reproduction:** Launch `target/debug/rivet ui --port 0`, submit the HTTP example with program port 0 through `/api/build`, and start its returned build with `/api/run`. Send SIGTERM to the workbench PID and wait for it. The parent exited with signal status `-15`; after 150 ms, a direct request to the child listener still returned `{"message":"hello world"}`. Observed parent PID 34696, child PID 34761, and ephemeral port 56697. The reviewer stopped the orphan with SIGTERM after confirming the result.

**Requested correction:** Route ordinary termination signals through the existing graceful shutdown path, for example by enabling the pinned `ctrlc` crate's `termination` feature, and add a CLI subprocess regression that terminates a running workbench with SIGTERM and verifies child exit plus immediate listener reuse. SIGKILL/process crashes are outside graceful-shutdown guarantees.

**Resolved.** The coordinator enabled `ctrlc`'s `termination` feature and added `sigterm_stops_workbench_and_releases_native_server_port`. The reviewer ran `cargo test --locked --test native_artifacts sigterm_stops_workbench_and_releases_native_server_port -- --exact`: the regression passed.

A separate raw-client retest additionally held a request open after declaring a 2,048-byte body and sending only one byte. SIGTERM then produced workbench exit 0 in 17.8 ms while that client remained open. The child PID no longer existed, both listener addresses refused new connections, the native service port rebound using the runtime's `SO_REUSEADDR` policy, and the artifact file remained on disk. This resolves the orphan finding. An ordinary socket without address reuse can still encounter TCP TIME_WAIT after shutdown; the evidence does not claim unconditional immediate rebinding of the workbench port.

## Independent checks performed

- `cargo test --locked --test native_codegen --test native_artifacts --test workbench`: all 15 tests passed (8 native-codegen, 4 native-artifact, 3 workbench).
- `python3 tests/native_runtime/run.py --sanitize undefined`: passed native values, limits, HTTP framing, shutdown, and rebind under UBSan.
- An additional temporary Python differential experiment, deterministic seed `20261025`, generated 12 distinct programs exercising every current operation. Each served nested objects containing safe generated signed additions, i64 extrema, both booleans, empty objects, text aliasing/conversion/concatenation, Unicode, embedded NUL, quote/backslash/newline and other control escapes, and sorted escaped keys. For each case the reviewer ran `rivet interpret`, built and ran both `baseline` and `size` artifacts, and compared two HTTP responses byte-for-byte to the reference. All 24 native builds and repeated comparisons passed; every experiment server exited zero on SIGINT. Temporary programs/artifacts were removed afterward.
- The targeted artifact suite independently exercised exact executable download bytes, copied artifact execution in an empty directory after workbench shutdown and original-artifact removal, cleared environment/nonexistent PATH, repeated standalone server requests, nonzero backend failure without an accepted artifact, and refusal to overwrite an existing output.
- Reviewed failure-order tests cover unused positive/negative checked overflow, a preceding text-size failure, console newline bounds, exact/overflowing JSON bounds, and escaped JSON expansion under both profiles. These tests passed in the review run above.

## Observations and limits

- String lengths remain explicit through the ABI; generated bytes are hexadecimal escaped; C JSON encoding preserves UTF-8/NUL and exact signed integer decimal strings. Key comparison is bytewise and consistent with the reference's UTF-8 string ordering. The generated IR invokes potentially failing support calls in source order, including unused instructions.
- Native allocations have process lifetime and are freed on normal exit or `rv_fail`; object indexing uses explicit maximum alignment. Per-value bounds do not form an aggregate hostile-program memory quota. The reviewed ABI assumes generated, well-typed callers, as documented.
- Native HTTP uses loopback, bounded nonblocking reads/writes, fixed-size body discard storage, and signal-aware deadlines. Existing strict runtime checks and UBSan are valuable; ASan was unavailable on the host according to the runtime handoff and is not claimed as passing.
- Build/run/download and status UI source correctly consume native artifact metadata, code views, OS PID, exact response bytes, and exit diagnostics. The reviewer inspected browser coverage but did not claim an independent interactive browser session; UI-owner and coordinator acceptance evidence is separate.
- Final clean-checkout reproduction, efficiency measurements, integrated browser acceptance, updated current-state documentation, and milestone/publication evidence remain the coordinator's responsibility. Existing historical documents must not be mistaken for native acceptance.

## Final review status

No unresolved implementation blocker remains after the shutdown correction and independent retest. The native architecture and reviewed semantic/lifecycle paths satisfy their specified behavior. Final integrated acceptance and publication remain subject to the coordinator checks listed above.
