# Project state

State version: 1. Updated: 2026-10-25.

## Milestones

| Milestone | Status | Required evidence |
| --- | --- | --- |
| Bootstrap brief and public repository | In progress; remote creation blocked | Initial commit pushed to a new public repository |
| MVP 1: language, runtime, Hello world UI | Not started | Compiler/runtime tests, changed-output check, invalid-program check, browser verification |
| MVP 2: program-defined local HTTP server | Not started | Repeated requests, edited response, stop/restart and clean shutdown, browser verification |
| Final reproducibility and handoff | Not started | Clean checkout build/test, exact launch commands, final/milestone commit links |

## Bootstrap evidence

- The starting workspace was empty, with no Git repository or implementation.
- Applicable global machine instructions were read before implementation; they are intentionally not copied here.
- Development target inspected: macOS 26.5.2, arm64; Rust/Cargo 1.90.0; Node 22.7.0; Python 3.11 available.
- Centralized GitHub App authentication and identity guard passed. The actor was confirmed as an App installation token with access to the intended owner's existing repositories.
- Chosen candidate repository: `mrmcgrewx/rivet-lang`.
- Repository creation via `POST /user/repos` returned HTTP 403 with `Rate Limit Exceeded`. The general REST quota still had over 4,990 requests remaining; the response supplied no retry delay. Both default and current API-version requests returned the same error. Do not infer that this means ordinary API quota exhaustion or a known missing permission.
- A retry after more than two minutes of backoff returned the same error. App access needs external resolution, or the owner can create an empty public repository and make it accessible to the App.
- The brief, initial architecture, contributor instructions, and state were committed locally before implementation. See Git history for the bootstrap commit; no checkpoint has been pushed.
- No substantive implementation or tests have run. No remote repository is confirmed created.

## Next steps

1. Resolve the remote-creation blocker: the owner can create an empty public `mrmcgrewx/rivet-lang` repository accessible to the App, or resolve App repository creation and authorize resuming the existing request. The backoff retry has already failed. Do not switch to personal credentials.
2. Once the repository exists, connect `origin`, push the initial brief/state checkpoint, and verify public visibility and remote commit.
3. Specify and commit the language semantics, module boundaries, workbench API, test strategy, and delegated file ownership. Then implement compiler/runtime and UI in parallel where useful.
4. Integrate and validate each MVP, recording and pushing its milestone commit.
5. Verify a clean checkout and actual browser UI flows before final delivery.

## Known limitations

This checkout currently contains project documentation only. There is no compiler, runtime, UI, example program, build configuration, or benchmark yet. Setup and launch instructions cannot be validated before implementation.
