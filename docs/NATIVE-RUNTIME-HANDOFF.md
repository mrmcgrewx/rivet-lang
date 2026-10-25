# Native support runtime handoff

Completed 2026-10-25 against the ownership and ABI in `NATIVE-PLAN.md`, committed before delegation in `bddf7f0`. This contribution changes only `native/runtime.c`, `native/runtime.h`, `tests/native_runtime/`, and this handoff. The coordinator owns integration and Git operations.

## Implementation

`native/runtime.h` exposes exactly native support ABI v1. `runtime.c` is C11 and links only ordinary system libraries. It does not read source, parse Rivet programs, execute bytecode, or dispatch language operations. Generated LLVM calls the helpers directly; signed addition and its overflow branch remain generated code.

- Immutable byte values retain explicit lengths and preserve embedded NUL and UTF-8. Text, concatenation, integer/boolean text conversion, and newline output observe the reference limits. Console output fails before writing when text plus newline exceeds 65,536 bytes.
- JSON uses exact signed i64 decimal output, serde-compatible control/quote escaping, and deterministic unsigned byte key ordering. Objects preserve nested JSON bytes. Duplicate helper keys use the last supplied value, matching the validator's already-deduplicated source objects. Object size is checked incrementally in sorted field order before result allocation.
- Escaped Text-to-JSON wrappers may temporarily exceed 64 KiB; the containing object accounts for their full serialized size, matching the interpreter's error order. An encoding is cached per immutable Text so repeated field references do not duplicate large escaped buffers. Helper indexing storage has explicit `max_align_t` alignment.
- Support allocations are process-lifetime and released by `rv_cleanup` on normal return, or by `rv_fail` on a runtime failure. Calling cleanup twice is safe. There is no per-request heap allocation.
- HTTP binds IPv4 `127.0.0.1`, supports port zero, and prints/flushed exactly the agreed readiness line. Route bodies are computed before serving, with program-defined status and bytes. Matching ignores a query suffix; other methods return 405 with `Allow: GET`; missing routes return 404. Responses include exact byte Content-Length, JSON content type, no-store, and connection close.
- Requests have an 8 KiB header limit, 64 fields, 2,048-byte ASCII target limit, and 512 KiB body limit. Duplicate Content-Length/Host, invalid framing, Transfer-Encoding, and Expect are rejected. Nonblocking sockets and monotonic deadlines bound the complete request read to two seconds and response writing to 250 ms. Body bytes are discarded through a fixed 4 KiB buffer after framing validation.
- SIGINT/SIGTERM interrupt both idle listeners and incomplete clients, then close sockets before return. SIGPIPE is ignored; an abandoned HTTP client does not terminate the server, while a failed console write reports a nonzero runtime error. Only `RIVET_CONTROL=1` produces `RIVET_REQUEST` lines on stderr, once per handled request, including rejected requests. Ordinary server stderr is silent.

## Checks actually run

Environment: Apple Clang 17.0.0 (`clang-1700.4.4.1`), target `arm64-apple-darwin25.5.0`, Python 3.11.9, macOS 26.5.2.

From the repository root:

```sh
clang -std=c11 -Wall -Wextra -Wpedantic -Wconversion -Wshadow -Werror -fsyntax-only native/runtime.c
python3 tests/native_runtime/run.py
python3 tests/native_runtime/run.py --sanitize undefined
```

All passed. The runner builds separate helper and HTTP fixtures using strict warnings, both `-O0` and `-Oz`. The UBSan run uses `-O1 -g -fsanitize=undefined -fno-omit-frame-pointer -fno-sanitize-recover=all`. Temporary executables are removed after the run.

Covered behavior:

- Empty values/objects; concatenation; exact i64 extrema; bool text/JSON; text immutability; Unicode, NUL, all short JSON control escapes, generic control escaping; deterministic key order, duplicate keys, and nested objects.
- Exact 65,536-byte JSON success; exact 65,536-byte console output success including newline; over-limit concatenation, console, JSON, and heavily escaped JSON failure with required byte counts and no stdout; exact NUL console output; closed stdout pipe failure.
- Repeated real TCP requests and exact raw JSON bytes; HTTP 201/404/405; query matching; exact maximum route and overlong target; exact 65,536-byte response and Content-Length.
- Exact 512 KiB request body acceptance; excessive Content-Length, duplicate framing, Transfer-Encoding, Expect, malformed header names, NUL header, too many fields, oversized headers; two-second unfinished-body expiry; abruptly disconnected response clients; live-listener port conflict.
- SIGINT and SIGTERM success; immediate same-port rebind; incomplete header and incomplete body shutdown in under 500 ms with the client still open; no default logging and exact opt-in control logging.

AddressSanitizer was attempted with `python3 tests/native_runtime/run.py --sanitize` (ASan + UBSan). Its executable spun before entering `main`. A one-second process sample showed Apple's ASan initialization calling `InitializeShadowMemory`, `get_dyld_hdr`, the intercepted allocator, and `StaticSpinMutex::LockSlow` recursively. The process was stopped and UBSan was run separately successfully. **ASan validation is unavailable on this host, not passed.** The runner now bounds helper execution to ten seconds, so a repeated sanitizer-startup failure cannot hang indefinitely. The diagnostic sample was not retained in the repository.

## Integration notes and limits

Compile generated LLVM together with `runtime.c` and make `runtime.h` available beside it, as specified in the plan. Both files may be embedded in the Rust compiler. No additional include/library path is needed. Native `main` must call `rv_cleanup` after the console entry or `rv_serve` returns.

This runtime assumes the compiler supplies valid UTF-8 and well-typed, correctly sized ABI arrays; it is not a memory-safe interface for arbitrary foreign pointers. Per-value limits are not an aggregate hostile-program memory quota. The local HTTP server is sequential, closes each connection after one request, rejects chunked requests, and has no TLS or streaming. Oversized requests are closed without waiting to drain unread input; a peer may observe a connection reset after the small error response. Methods longer than 63 bytes are rejected as malformed. No claim of production network hardening or general fuzzing coverage is made.

The coordinator still needs to run generated-artifact differential tests, verify the integrated workbench and standalone copied executables, record efficiency measurements, and complete actual browser acceptance. Those checks are not claimed by this runtime handoff.
