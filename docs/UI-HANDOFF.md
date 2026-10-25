# Workbench UI handoff

Updated 2026-10-25. Ownership and API assumptions follow the committed `docs/CONTRACT.md` at `e4e6b01`. This contribution changes only `web/`, `tests/ui/`, package files, the Playwright configuration, and this document. No agent-local Git mutations were performed; the coordinator integrates and commits these files.

## Implementation

- `web/index.html`, `web/style.css`, and `web/app.js` provide an offline, dependency-free engineering workbench. Rust embeds these assets, so rebuild the executable after changes. No frontend compilation, font fetches, CDN assets, or inline scripts/styles are required.
- The workbench includes the API-provided examples; new/open/download; a labelled, editable source area with line numbers; explicit Build, Run/Start, and Stop; compiler/runtime diagnostics; standard output; typed IR; server address/build/request count/routes; and a real HTTP request inspector.
- The editor, built artifact, and running server are distinct states. Source changes disable Run until the exact current source is built. The running server retains its build ID across source edits, successful new builds, and failed builds. Stop remains enabled when the editor is invalid. A status response from before a mutation cannot overwrite its result.
- Console results identify their build. Server status refreshes once per second while the page is visible. Requests always go through `/api/request` and display the returned status, content type, time, and **exact raw response body**. Raw display intentionally avoids JavaScript numeric rounding of valid signed 64-bit Rivet values.
- Successful requests against an older running server preserve current compiler diagnostics. Native Tab leaves the source editor; the result tabs support arrow keys, Home, and End. Build supports Command/Ctrl + Enter. The narrow layout retains file actions in its horizontally scrollable program bar.
- The interface never supplies demonstration output or responses: those values come from the backend. Only the empty new-program template is created in the browser.

## Validation actually performed

Environment: macOS arm64, Node 22.7.0, npm 10.8.2, pinned `@playwright/test` 1.64.1, Chromium 156.0.8078.4 (Playwright chromium v1248).

1. `npm install --ignore-scripts` generated the committed lockfile and reported zero vulnerabilities.
2. `npx playwright install chromium` installed the browser used below.
3. `node --check web/app.js` and `node --check playwright.config.js` passed.
4. Final `npm test` passed **8/8** real-browser tests in **6.0 seconds**, using the actual Rust workbench and actual program TCP listeners. The suite starts its own workbench on 127.0.0.1:7332, with one worker and no reuse of an existing server. `npm run test:ui` is an alias. `RIVET_TEST_URL` can target an explicitly supplied existing workbench instead.
5. Rendered screenshots at 1440×1100 and 390×844 were inspected. The revised layout has legible program text, responsive panels, no horizontal page overflow, and visible execution controls. Temporary screenshot files are outside the repository. This is headless Chromium visual inspection; the coordinator's separately required interactive browser verification and clean-checkout reproduction are not claimed here.

The eight test scenarios cover:

- Hello world, nonempty typed IR, edited output, and stale-source Run prevention.
- Syntax/line-column errors, missing references, and execution-time overflow diagnostics.
- Repeated real HTTP requests, JSON content type/body, 404, counters, Stop releasing the listener, and restart.
- Failed build preserving a running artifact and its Stop control, diagnostic preservation during requests, then edited response and route after restart.
- Computed addition/conversion/nested JSON values.
- Exact display of `9223372036854775807` in an HTTP body.
- New/open/download and source-file roundtrip.
- Keyboard tab navigation, editor focus exit, 390px layout, and browser script-error checks during interaction.

## Limits and follow-up context

- Editor contents are in memory; reloading opens the first example. Download before leaving if source should be retained. The runtime's existing server can still be stopped after reload; a fresh build is required before running the newly loaded editor content.
- The editor is a native textarea with line numbers, not a syntax-highlighting or autocomplete system. The UI intentionally keeps the explicit build step and typed IR observable.
- Verification is Chromium-only on the development machine. There is no claim of a comprehensive accessibility audit or support verification on other browsers/platforms.
- Initial example-load failure requires a page refresh after the runtime is restored. The status badge does recover from temporary status polling failures.
- The test suite mutates the workbench it targets and stops its program server before and after each test. The default isolated test port avoids the normal workbench port 7331.

Coordinator remaining work: guarded milestone commits/pushes, actual browser verification, clean-checkout reproduction, and updates to shared state/architecture/final documentation.
