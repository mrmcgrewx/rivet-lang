# Independent correctness review

Reviewed 2026-10-25 under the review ownership committed in `72ca645`.
Implementation review complete: the actionable findings below have been corrected.
Final clean-checkout reproduction and visual acceptance remain the coordinator's responsibility.
The reviewer owns this document only and made no production or Git changes.

## Scope and evidence

- Read `AGENTS.md`, `docs/BRIEF.md`, `docs/ARCHITECTURE.md`, `docs/STATE.md`, and `docs/CONTRACT.md` before review.
- Inspected all Rust implementation, language/runtime/workbench tests, UI source, browser tests, and language semantics documentation available during review.
- Ran `cargo test --locked`: 2 runtime tests, 22 language tests, and 3 workbench tests passed at the first review snapshot. Later test additions and integrated validation are separate evidence.
- Used disposable local CLI processes and raw loopback TCP clients for the lifecycle reproductions below. Closed clients and stopped all experiment processes afterward.
- Confirmed the implementation is a genuine compiler/interpreter: validation resolves symbolic operands to private typed indexed register instructions; execution evaluates every instruction; console/HTTP entries consume computed values. The frontend receives executed results from the API and does not implement either demonstration's output.

## Findings and resolutions

### P1: An unfinished request body prevents Stop and Ctrl-C shutdown

**Locations at initial review:** `src/workbench.rs:166-169`, `src/runtime.rs:87` (`request.respond`), and the `tiny_http` request cleanup behavior.

**Reproduction A — workbench:** Start `target/debug/rivet ui --port 0`. Connect a raw TCP client to its reported address and send an authorized `POST /api/build` with `Content-Type: application/json`, `X-Rivet-Request: 1`, `Content-Length: 2048`, and only `{` as the body. Leave the socket open and send SIGINT to Rivet. The process remained running after 700 ms and exited immediately only after closing the client.

**Reproduction B — program server:** Build/start the server example with port 0. Send a raw `GET /hello` with `Content-Length: 2048` and only `x` as its body. The client receives `HTTP/1.1 200 OK`. Then call `/api/stop`: it does not return within a 1-second client timeout, and completes only after closing the unfinished program request socket.

The workbench performs a blocking body read on its only service thread. More subtly, `tiny_http`'s `EqualReader::drop` drains unread declared bytes even when application code did not explicitly read the body. Thus responding early or rejecting an oversized body does not fix the hang. The same destructor allocates a buffer based on the remaining declared length; application body limits do not bound that cleanup allocation. The cleanup allocation issue was established by dependency source inspection, not by attempting a destructive huge allocation.

**Resolved.** The coordinator replaced `tiny_http` with an owned socket adapter that bounds headers/body, polls cancellation while reading, limits read/write time, and closes each connection without draining an abandoned body. Independent CLI/API retests against the replacement measured Ctrl-C exit 0 in 42.7 ms with the incomplete workbench client still open, and `/api/stop` completion in 25.9 ms with the incomplete program client still open. Rebuilding/restarting Rivet on the same program port succeeded immediately and returned HTTP 200. A declared `Content-Length: 1000000000000` returned 413 without sending a body; Transfer-Encoding returned 501; duplicate Content-Length and duplicate Origin returned 400. The workbench remained responsive afterward.

The new transport's listener is directly owned by the serving thread, and Stop joins that thread before returning. Source review confirms this also removes the former asynchronous `tiny_http` listener-drop race. `cargo test --locked` passed the new 3 transport regressions, 2 runtime tests, 23 language tests, and 3 workbench tests during re-review.

### P2: HTTP inspector rounds valid 64-bit integer response values

**Location at initial review:** `web/app.js`, request form handler's `JSON.parse(result.body)` followed by `JSON.stringify`.

Rivet permits the complete signed 64-bit integer range. Its HTTP JSON body can therefore contain `9223372036854775807`, while JavaScript's ordinary JSON parser rounds this to `9223372036854776000`. The inspector's formatting step changes the displayed response and can mislead the engineer inspecting computed output.

**Resolved.** The UI now assigns `result.body` directly to the inspector's `textContent`. The new browser regression provides raw source containing the boundary integer and asserts the exact response `{"max":9223372036854775807}`. The UI owner reported all 8 browser tests passed after this fix. The reviewer inspected the fix and regression; browser execution is attributed to the UI owner rather than claimed as an independent reviewer run.

### P2: Accepted route lengths exceeded the HTTP transport limit

The compiler originally accepted route paths up to the source-size limit, while the request helper and replacement transport limit targets to 2048 bytes. Consequently a program could compile an endpoint that could not be served.

**Resolved.** The compiler now rejects route paths over 2048 bytes with a route-field diagnostic; the committed contract documents the same bound. The reviewer ran `cargo test --locked --test language route_path_limit_matches_transport_at_exact_byte_boundary -- --exact`: one test passed, exercising acceptance at 2048 bytes and rejection at 2049 bytes. The complete language suite contains 24 tests after this addition.

### Reproducibility command mismatch

README and CI initially invoked `npm test` while the package offered only `test:ui` scripts. **Resolved:** `package.json` now defines `"test": "playwright test"`, matching the documented and CI commands.

## Review observations that passed

- Build-time unknown fields, invalid types, duplicate instruction IDs, missing/forward references, invalid HTTP definitions, and syntax diagnostics have direct automated coverage.
- Runtime overflow is distinct from compilation; unused instructions execute and can fail. Output is returned only after successful evaluation.
- Text concatenation checks its final size before allocating; JSON construction accounts for escaped keys/values and nested serialized size. Limits are documented as per-value prototype bounds, not a complete hostile-program sandbox.
- Build artifacts and running servers are separate. Failed builds clear the current artifact while the prior server stays available; Stop does not depend on editor validity. UI marks edited source separately from the built source.
- The request inspector sends a real TCP HTTP request to the running server. The API does not accept an arbitrary destination host/URL.
- Current workbench tests verify successful edited console and HTTP output, repeated requests, stale build rejection, running-server preservation across build failure, and origin/host rejection.
- Browser tests cover both MVP paths, source changes, diagnostics, repeated HTTP calls, stop/restart, computed values, and file roundtrip. The reviewer has read these tests; execution/visual verification is recorded by the UI owner and coordinator.

## Final review status and limits

- No unresolved implementation blocker was identified in this review after the corrections above.
- `cargo fmt --check` passed at the re-review snapshot.
- The bounded HTTP subset handles one request at a time per listener, rejects chunked transfer, and allows a slow client to occupy a listener up to its two-second read deadline. These limits are now documented in `docs/ARCHITECTURE.md` and are acceptable for this local prototype.
- Normal compiler/runtime/API tests plus explicit raw-client experiments establish correctness for the reviewed paths; they are not a general fuzzing or hostile-network security audit.
- The coordinator must still record the final integrated test results, actual browser interaction, clean-checkout build, milestone commits, and publication evidence before declaring the project complete.
