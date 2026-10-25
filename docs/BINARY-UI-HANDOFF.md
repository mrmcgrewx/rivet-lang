# Binary workbench UI handoff

Updated 2026-10-25. Implemented within the ownership and binary protocol committed in `372d143` / `BINARY-PLAN.md`. Changes are limited to `web/`, `tests/ui/workbench.spec.js`, and this handoff. No Git mutations were performed by this agent.

## Source and editing model

The canonical browser state is a `Uint8Array` containing the actual RVB1 source file. Examples are loaded as raw bytes from the metadata-provided URL. Open reads `File.arrayBuffer()`. Download binary source writes a Blob with `application/octet-stream` and `.rvb` filename; it never saves the decoded display or hex characters as source. Build and Inspect post exactly those bytes with `application/octet-stream` and `X-Rivet-Request: 1`. JSON is used only for API commands and metadata/inspection responses.

The source panel has two explicitly labelled views:

- **Decoded program** is read-only text supplied by the binary inspector. It shows operations and register references without making that text the authoring format.
- **Hex bytes** edits the source bytes directly. Hex pairs and whitespace decode into the byte buffer, then the backend validates the binary after a short debounce. Invalid hex is distinguished from a valid byte sequence with an invalid RVB1 structure. The gutter reports cumulative byte positions even if rows contain fewer than sixteen bytes.

Literal controls use inspected byte offsets to splice a Text, Int or Bool payload and update its record length and the header's body length. Signed integers use JavaScript `BigInt` and `DataView.setBigInt64`; they never pass through floating-point JSON numeric conversion. Boolean edits encode one byte. Text edits count UTF-8 bytes and enforce the literal and source limits. HTTP port editing writes the entry's little-endian u16 field. New binary source starts from a fixed 22-byte RVB1 program containing an empty Text literal and console entry. No source JSON parser, JSON assembler, or textual assembler is present in the UI.

Changing instruction structure, references, routes or status uses the hex view. The decoded literal controls deliberately cover only scalar payloads and the HTTP port. Each byte edit triggers fresh inspection, invalidates the previous build when bytes differ, and preserves any independently running native server. The selected literal survives reinspection. Stale inspection results are discarded by source revision, and API status races use the existing epoch guard.

## Preserved native behavior

The existing executable card, native download, typed/LLVM/assembly code views, exit diagnostics, process IDs, request inspector, and Stop behavior remain. Source download and executable download are distinct controls and formats. Native Run is enabled only when the current source bytes equal the accepted build's byte snapshot. An invalid build clears the current downloadable executable while an existing server remains stoppable. Unexpected native exits are appended once without replacing current binary diagnostics. HTTP response bytes are displayed without parsing/reformatting their signed integers.

## Verification actually performed

- `node --check web/app.js` — passed.
- `node --check tests/ui/workbench.spec.js` — passed.
- `git diff --check -- web tests/ui` — passed.
- `npm test` — **12/12 Chromium scenarios passed** against the real binary API/native compiler and OS processes. The final run after the gutter correction completed in 19.9 seconds. The suite starts its own workbench on port 7332 and stops it afterward. No source/build API mocks are used.

Coverage includes:

1. Read-only decoded inspection, editable hex, exact original binary download, non-JSON file bytes, irregular-row offset labels.
2. Hello native execution, UTF-8 text byte edits, binary length updates, and rebuild requirements.
3. Native artifact metadata, all three compiled code views, executable byte equality/header, and distinction from RVB1 source download.
4. Incomplete hex, invalid magic, byte diagnostics, JSON-source rejection, and directly edited opcode-file payload bytes.
5. Valid binary checked-overflow failure and native exit status.
6. HTTP port edit, repeated real requests, missing route, PID, stop/restart and released listener.
7. Failed binary build while an old server runs, followed by changed source taking effect after restart.
8. Computed binary example with integer/boolean edits and nested JSON response.
9. Exact i64 maximum/minimum byte roundtrip, exact HTTP response, and rejected out-of-range edit without source mutation.
10. Fixed-byte New, binary import/export exactness, and native output from the imported file.
11. Unexpected child exit while preserving compiler diagnostics, without duplicate errors on later polls.
12. Result-tab keyboard navigation, both source views at 390px width, native build/run, no horizontal page overflow or page errors.

The independent reviewer noted that a fixed sixteen-byte gutter was inaccurate for arbitrary line formatting. That issue was corrected to use cumulative preceding digits and is now covered by the first test.

## Visual inspection

An additional temporary Playwright capture script launched a separate ephemeral-port workbench, verified Hello output, switched decoded/hex views, started a binary HTTP program and sent a real request, then stopped the child and workbench. Captured desktop views at 1440×1100 and a compact 390×844 view; reviewed the resulting full-page images. Paths are `/tmp/rivet-binary-ui-decoded.png`, `/tmp/rivet-binary-ui-hex.png`, `/tmp/rivet-binary-ui-http.png`, and `/tmp/rivet-binary-ui-mobile.png`. These are temporary QA captures, not the coordinator's required live in-app-browser acceptance evidence.

The byte/source identity, readable projection, literal edits, native artifact metadata and HTTP response remain distinct and readable. Long artifact paths wrap in the compact layout; the program library scrolls horizontally within its own strip. Full-page captures are taken from scroll position zero so the existing offscreen skip link does not appear as a capture artifact.

## Integration status

No unresolved UI blocker is known. All interfaces match the committed binary plan. Coordinator responsibility remains actual in-app-browser acceptance, combined review/clean-checkout validation, current shared documentation, and guarded Git publication. No test workbench or native server is intentionally left running by this agent.
