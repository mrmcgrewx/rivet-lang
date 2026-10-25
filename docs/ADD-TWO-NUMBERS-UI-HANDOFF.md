# Add Two Numbers UI handoff

UI scope follows `ADD-TWO-NUMBERS-PLAN.md`, committed as `8a6b220` before delegation. Changes are limited to `web/`, `tests/ui/`, and this handoff. No Git operations or backend/compiler/runtime edits were made by the UI owner.

## Behavior

The fifth example uses the existing compute input/build/run path. General help now covers both arrays and linked lists. Add Two Numbers has its own contextual help explaining that each input list starts with its length, digits begin at the ones place, inputs have 1–100 digits from 0–9, and a carry can add an output digit. It also explains the no-leading-zero rule in the displayed reverse digit order. Two Sum retains its separate array/target guidance; the input field's description references only the active example's help.

The UI consumes the example's default input from backend metadata, and displays results returned by the native executable. It does not solve the problem, convert full numbers, or synthesize results. Runtime input changes preserve source bytes and the current build. The existing raw binary open/save/hex paths, native artifact details and input error recovery are retained.

## Verification

Performed on 2026-10-25 against the integrated working tree:

- `node --check web/app.js`, `node --check tests/ui/workbench.spec.js` and `git diff --check -- web tests/ui` passed.
- `npm test`: **16 real Chromium scenarios passed in 24.7 seconds**, against the actual workbench, compiler and native runtime on port 7332. No compiler or runtime response mocks are used.
- A separate browser run loaded the fifth example, compiled its 657-byte source, ran the default input and displayed `[7,0,8]` with exit code 0. Desktop 1440×1100 and compact 390×844 screenshots were inspected: contextual help, source, output, runtime input and build/artifact controls remain readable without horizontal page overflow. Temporary captures are ignored outputs under `.rivet/`; the temporary workbench on port 7334 was stopped.

Two new scenarios bring the full Chromium suite to 16. They build Add Two Numbers once and run [all three published cases](https://leetcode.com/problems/add-two-numbers/description/) plus a 100-digit carry chain, check submitted build IDs and unchanged executable path, download exact source bytes before/after input changes, inspect hex and compact layout, recover from an invalid-digit assertion without rebuilding, and verify that switching to Two Sum restores its default input and contextual help.

No unresolved UI blocker remains. The coordinator owns final combined/clean verification, actual in-app browser acceptance, shared documentation and publication.
