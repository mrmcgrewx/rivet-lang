# RVB2 computation UI handoff

The UI-owned work follows the contract in `TWO-SUM-PLAN.md` (committed as `fe95cd0`). Changes are limited to `web/`, `tests/ui/`, and this handoff. The coordinator owns compilation/runtime/API integration, shared documentation, actual in-app browser acceptance and all Git operations.

## Behavior

Compute entries display a separate Runtime input panel. Loading an example initializes the panel from its metadata `input` field; importing a file or creating a new source clears runtime input. The Two Sum example explains its count-prefixed array, target and zero-based result indices. Other compute files show generic integer/array input help. Runtime input is sent as a string only for compute runs, preserving integers beyond JavaScript's exact Number range.

Input changes keep the existing binary source and native build valid. After a run, changing input prompts the user to run the same executable again. Source editing still requires rebuilding when bytes change. File downloads remain exact `.rvb` bytes, excluding runtime input. Native artifact metadata/download and generated-code views remain unchanged. RVB1 console and HTTP runs omit the input field entirely.

The input panel counts actual UTF-8 bytes and disables Run above 262144 bytes. Backend validation remains authoritative for integer syntax, runtime errors and bounds. Inputs can be repaired and rerun without rebuilding. Run is also disabled while source inspection is pending, preventing an identical-byte edit from temporarily losing compute entry metadata and launching without its input.

Source labels read the actual binary header version, so RVB2 is no longer labelled RVB1. The empty convenience-editor panel is hidden when compute inspection supplies no scalar literals; full hex editing remains available. Input controls use labels, contextual descriptions, keyboard focus styles and a compact layout.

## Verification

Performed on 2026-10-25 against the integrated working tree:

- `node --check web/app.js` and `node --check tests/ui/workbench.spec.js`: passed.
- `npm test`: all **14 Chromium scenarios passed** against a real workbench on port 7332. Playwright's final run record reports `passed` with no failed tests. There are no compiler/runtime mocks.
- Separate Chromium visual inspection at 1440×1100 and 390×844: loaded Two Sum, built its native artifact, executed the default input and observed `[0,1]` with exit code 0. Both rendered layouts were inspected; input, result, build controls and artifact remained readable without horizontal page overflow. Temporary captures are ignored outputs under `.rivet/`; the temporary workbench on port 7334 was stopped.

The browser suite adds two scenarios (14 total). They cover one native build reused for all three published Two Sum cases and an exact integer above 2^53, submitted input/build IDs, source byte preservation through input edits/download/hex view, automatic input hiding and omission for Hello, runtime-input failure recovery, UTF-8 byte limits, binary import with cleared input and compact-layout width.

Visual inspection found that the initial compute disassembly placed all 13 register declarations before any operations, filling the source viewport. The compiler owner agreed to compact those declarations into rows of four; this changes only readable inspection, not binary bytes or API metadata. No unresolved UI blocker remains. The coordinator still needs final integrated/clean verification and actual in-app browser acceptance after all owners' changes.
