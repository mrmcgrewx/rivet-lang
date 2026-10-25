# Native workbench UI handoff

Updated 2026-10-25. Ownership follows `docs/NATIVE-PLAN.md`, committed in `bddf7f0` before delegation. This contribution changes only `web/`, `tests/ui/`, and this handoff. No Git writes were performed by the UI agent.

## Implementation

- The existing Build / Run / Stop toolbar remains directly below the editor and results. Console Run is labelled **Run executable**; HTTP retains **Start server**. All outputs still come from the API; no demonstration behavior is implemented in the UI.
- A native executable card immediately below the toolbar exposes the absolute path, exact byte count, profile, target, and backend `build_ms`. Download uses the API-provided executable URL and names the attachment `rivet-program`. The card explains that artifacts persist after workbench exit and downloaded copies may need `chmod +x rivet-program`.
- **Compiled code** retains one result tab with a native selector for **Typed IR**, **LLVM IR**, and **Assembly**. It renders API `ir`, `llvm_ir`, or `assembly` as literal text and labels which build/source the view represents.
- Source, current build, and running server remain separate. Editing source marks the previous executable and disables Run. The previous artifact can still be downloaded until another build replaces it. A failed build clears the downloadable artifact without hiding the running server or its Stop action.
- Console success displays the actual exit code. A failed native run can return `exit_code`, `stdout`, and `stderr`; diagnostics remain selected, and Console records the process exit and stderr. A null exit code is shown as unavailable; the UI does not infer a termination signal from it. HTTP status adds the child PID. Polling notices when the managed child disappears and appends a newly reported `last_error` once, preserving existing compiler diagnostics.
- The HTTP inspector continues to assign the raw response body directly to `textContent`, preserving the exact signed 64-bit JSON values and bytes exposed by the API.
- Native metadata wraps in narrow layouts; file paths and target names cannot expand the page width. Code selection is a labelled native select, and the existing result-tab keyboard navigation is retained.

## Validation

Actually performed on macOS arm64 with Node 22.7.0 and pinned Playwright 1.64.1 / Chromium:

- `node --check web/app.js` — passed.
- `node --check tests/ui/workbench.spec.js` — passed.
- `git diff --check -- web tests/ui docs/NATIVE-UI-HANDOFF.md` — passed.
- `npm test` — **10/10 actual Chromium scenarios passed** after native backend integration. The suite started and stopped the actual Rust workbench on port 7332 and used real Clang builds, native child processes, artifact downloads, and TCP requests; no API mocks were used.
- Captured and inspected actual native Hello, HTTP, and LLVM views at 1440×1100, plus a 390×844 compact view. The executable card remains below the execution toolbar, long paths wrap, no horizontal page overflow occurs, and the HTTP PID/response are visible. Capture script also stopped its HTTP child and workbench afterward. Temporary screenshots were produced outside the repository; the coordinator's separately required interactive browser acceptance and durable evidence remain separate work.

The updated Chromium suite contains ten scenarios. In addition to preserving existing source editing, error, lifecycle, exact i64 response, file roundtrip, keyboard, and compact-layout coverage, it now checks:

- Backend artifact path, exact size, profile, target, and build time are shown.
- Typed IR, LLVM IR, and generated assembly match their API fields.
- Browser download bytes match the persisted file exactly and have a native executable header (Mach-O on macOS, ELF on the Linux test host).
- Console success and overflow failure show the reported native exit code.
- A running HTTP process exposes a positive PID, and Stop clears it.
- Sending SIGTERM externally to the test's native server updates its status to Stopped and appends its exit diagnostic once. Existing compiler diagnostics survive, and subsequent status polls do not duplicate the runtime message.
- Failed builds disable executable download while preserving the older running service.
- Modified source marks the existing artifact as previous source, and the metadata remains within a 390px viewport.

The coordinator owns the shared Playwright configuration, with test timeout 60 seconds and expectation timeout 15 seconds for real native builds.

## Integration assumptions and limits

The Build and Status API `build` values include `artifact { path, size_bytes, profile, target, download_url }`, `ir`, `llvm_ir`, `assembly`, and `build_ms`. Console Run returns `exit_code`; process failure responses include `exit_code` and `stderr` when known. HTTP server objects include `pid`. Status `last_error` records an unexpected native server exit and is cleared by a successful build/run. Download serves the saved executable's bytes.

The download test reads the reported artifact path from the local filesystem, so an explicitly supplied `RIVET_TEST_URL` must address a workbench with locally accessible artifacts. No dependency/package files, Rust, C, or shared documentation were changed. Coordinator validation still covers executable independence after workbench exit, a clean native build, actual interactive browser acceptance, milestone commits/pushes, and shared records.
