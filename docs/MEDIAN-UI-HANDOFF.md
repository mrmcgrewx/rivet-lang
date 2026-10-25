# Median UI handoff

UI scope follows `MEDIAN-PLAN.md`, committed as `cccf624` before delegation. Changes are limited to `web/`, `tests/ui/`, and this handoff. No Rust/C/shared-doc edits or Git mutations were made by the UI owner. The live workbench on port 7331 was left untouched.

## Behavior

The sixth example uses the existing compute input and native build/run path. Its contextual help explains both length-prefixed arrays, an empty array encoded as `0`, the nonempty combined input and per-array length bounds. It clearly identifies sorted order and the problem's integer range as caller preconditions. Search complexity is stated separately from linear input parsing, and scientific result notation is illustrated without changing the program's exact output.

Input-only changes preserve source bytes, current build and executable path. Runtime output remains the string returned by the native program; the GUI does not compute a median or parse/reformat floating-point results. Other examples retain their contextual guidance. The existing default size build profile is unchanged in the GUI; speed is a CLI capability for this milestone, with no benchmark dashboard added.

Long example names now wrap inside the desktop library column. The compact horizontal example strip retains single-line names and scrolling. Existing decoded/hex views, exact binary download/import and native artifact views remain intact.

## Verification

Performed on 2026-10-25 against the integrated working tree:

- `node --check web/app.js`, `node --check tests/ui/workbench.spec.js` and `git diff --check -- web tests/ui` passed.
- `npm test`: **18 real Chromium scenarios passed in 35.2 seconds**, using the workbench, compiler and native runtime on port 7332 without compiler/runtime mocks.
- A separate browser run loaded the sixth example, compiled its 803-byte source and ran a fractional case with exact output `2.5000000000000000e+00` and exit code 0. Desktop 1440×1100 and compact 390×844 screenshots were inspected. The long library name wraps correctly on desktop; input guidance, source, result and artifact controls remain readable without horizontal page overflow. Temporary captures are ignored `.rivet/` outputs; the temporary workbench on port 7334 was stopped.

Two new scenarios bring the suite to 18. They reuse one native build for odd, fractional, negative and both empty-sided cases; compare exact canonical scientific output; check submitted input/build IDs and source byte preservation through download/hex view; recover from empty-combined-input and malformed-token errors without rebuilding; and verify contextual help and compact layout.

No unresolved UI blocker remains. The coordinator owns final combined/clean verification, actual in-app browser acceptance, shared documentation, benchmark evidence and publication.
