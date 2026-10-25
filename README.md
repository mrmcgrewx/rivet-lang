# Rivet

[![CI](https://github.com/mrmcgrewx/rivet-lang/actions/workflows/ci.yml/badge.svg)](https://github.com/mrmcgrewx/rivet-lang/actions/workflows/ci.yml)

**An experimental binary programming language for machine-authored programs, with native compilation and a graphical inspector.**

Rivet programs are authored as **binary `.rvb` source files**. Numeric opcodes, type tags, register references, and byte payloads define the program. The compiler decodes those bytes directly, validates them, and produces a standalone native executable through LLVM and Clang.

A local browser workbench provides the human interface: decode the source, inspect its bytes, edit values, build, and run. The readable view is generated from the binary file. JSON and textual assembly are not required source formats.

**Status:** experimental prototype. Binary programs support console output, local HTTP services, and computations with runtime input, loops, arrays, integer maps, linked lists and floating-point values. Native output targets **macOS on Apple Silicon**. AI-generation reliability and broad performance competitiveness remain research questions; the [capability roadmap](docs/ROADMAP.md) makes those goals explicit.

## Source is binary; output is native

| File | Role |
| --- | --- |
| [`examples/hello.rvb`](examples/hello.rvb) | The authored program: 49 bytes of RVB1 instructions and entry data. |
| `hello` | The native executable produced from that source. |
| `hello.ll`, `hello.s`, `hello.build.json` | Optional inspection files produced during compilation. |

Opening `hello.rvb` in a text editor does not reveal textual source code. Its text literals may still be recognizable: binary encoding is not encryption. The compiler and GUI understand the numeric instruction format; a person can inspect it using the same decoder.

The committed examples are written directly from opcode bytes, without compiling JSON into them. [`scripts/write_binary_examples.py`](scripts/write_binary_examples.py) is an optional reproducibility tool for writing those bytes, not a program needed to execute the examples. The `.rvb` files are the source of truth.

## Quick start

### Requirements

- **macOS on Apple Silicon** — the verified native target.
- **Rust via [rustup](https://rustup.rs/)** — Rust 1.90.0 is pinned.
- **Apple's Xcode command-line tools** — install with `xcode-select --install` if needed. Native builds require Clang; verification uses Apple Clang 17.0.0.

```sh
git clone https://github.com/mrmcgrewx/rivet-lang.git
cd rivet-lang
cargo build --release --locked
./target/release/rivet build examples/hello.rvb -o ./hello
./hello
```

Output:

```text
Hello, world!
```

`hello` is an arm64 Mach-O executable. It runs without Rivet, Clang, the source file, inspection files, or a workbench process; on the verified target it links only to the macOS system library. Existing output files are not overwritten: choose another output path when rebuilding. `RIVET_CLANG` can select another compiler executable.

### Inspect the source

```sh
./target/release/rivet inspect examples/hello.rvb
xxd -g 1 examples/hello.rvb
```

The first command decodes a readable view. The second displays the actual source bytes in hexadecimal:

```text
52 56 42 00 01 00 00 00 03 00 01 00 21 00 00 00
01 01 07 00 48 65 6c 6c 6f 2c 20 01 01 06 00 77
6f 72 6c 64 21 10 01 06 00 02 00 00 00 01 00 02
00
```

These are 49 binary bytes, not a file containing hexadecimal text. They define two text values, a concatenation, and a console entry. The full [binary format specification](docs/BINARY-FORMAT.md) defines every byte, including the header, numeric operations, operand layout, and error rules. It includes direct binary authoring instructions.

### Browser workbench

```sh
./target/release/rivet ui
```

Open [localhost:7331](http://127.0.0.1:7331/). Load **Hello world** or open a `.rvb` file, inspect its decoded instructions and raw hex bytes, then **Build** and **Run executable**.

The workbench supports binary source import/download, scalar literal editing, HTTP port editing, raw byte editing, byte-offset diagnostics, and inspection of typed IR, LLVM, and native assembly. Builds expose an executable path and download. Server controls show the native process and make real HTTP requests.

**Download source** saves binary `.rvb` bytes. **Download executable** saves the compiled native program. Source edits, accepted builds, and running servers have separate state; editing or rebuilding does not change a running server. Executable artifacts persist under `.rivet/builds/` after the workbench closes. Save source before reload; the editor and UI build history are held in memory. Downloaded executable copies may need `chmod +x rivet-program`.

Use `./target/release/rivet ui --port 0` for an available port. Ctrl-C or SIGTERM stops the workbench and its managed server.

### Two Sum: a binary algorithm with runtime input

[`two-sum.rvb`](examples/two-sum.rvb) is a **231-byte RVB2 program** solving [LeetCode Two Sum](https://leetcode.com/problems/two-sum/description/). Its 21 instructions read an array and target, loop over the elements, look up each complement in an integer map, and return two distinct indices.

```sh
./target/release/rivet build examples/two-sum.rvb -o ./two-sum
printf '4 2 7 11 15 9\n' | ./two-sum
# [0,1]
printf '3 3 2 4 6\n' | ./two-sum
# [1,2]
printf '2 3 3 6\n' | ./two-sum
# [0,1]
```

Input is **element count, elements, target**, separated by whitespace. In the first line, `4` is the count, `2 7 11 15` is the array, and `9` is the target. These numbers are runtime data; the binary program stays unchanged. The same executable handles every case without rebuilding.

In the workbench, select **Two Sum**, inspect its decoded operations or binary bytes, and Build. Edit **Runtime input** and Run again to try another case. Input changes preserve the build; source changes require a new one. `rivet run` and `rivet interpret` also accept the same data on stdin.

The algorithm has expected O(n) time using generic hash-map operations. Generated native branches perform the loop and decisions; C support provides array/map storage and input/output. There is no Two Sum helper in the runtime. The current bounded map reserves a fixed 32,768-slot table and uses at most 10,000 entries. See the [algorithm walkthrough](docs/TWO-SUM.md) and [RVB2 specification](docs/COMPUTE-FORMAT.md).

### Add Two Numbers: linked lists and large-number arithmetic

[`add-two-numbers.rvb`](examples/add-two-numbers.rvb) is a **657-byte, 70-instruction program** using real linked nodes. It adds two numbers represented as lists of reversed decimal digits, including 100-digit inputs and 101-digit results:

```sh
./target/release/rivet build examples/add-two-numbers.rvb -o ./add-two-numbers
printf '3 2 4 3 3 5 6 4\n' | ./add-two-numbers
# [7,0,8]
```

Input contains two count-prefixed lists: `3 2 4 3` is `[2,4,3]` (342), and `3 5 6 4` is `[5,6,4]` (465). The output list `[7,0,8]` represents 807. In the workbench, select **Add Two Numbers** and reuse its build while changing Runtime input.

The program traverses nodes, carries digits and constructs the result in native control flow. It runs in O(max(m,n)) time. This exercise adds general linked-list operations, typed reference copying, checked division/remainder and source assertions; none is specific to addition. See the [algorithm and tradeoffs](docs/ADD-TWO-NUMBERS.md).

### Median of Two Sorted Arrays: logarithmic search and fractional results

The **803-byte, 81-instruction** [`median-two-sorted-arrays.rvb`](examples/median-two-sorted-arrays.rvb) searches for the middle partition without merging the arrays:

```sh
./target/release/rivet build examples/median-two-sorted-arrays.rvb -o ./median --profile speed
printf '2 1 2 2 3 4\n' | ./median
# 2.5000000000000000e+00 (2.5)
```

Input is two count-prefixed sorted arrays. The workbench has the same example and accepts different inputs using one build. Search takes O(log(min(m,n)+1)) time and O(1) auxiliary storage; reading/storing stdin adds O(m+n). Sortedness and the problem's element range are caller preconditions. This exercise adds reusable finite binary64 values, conversions, arithmetic and scalar output. See the [algorithm walkthrough](docs/MEDIAN.md), [runtime/memory comparison results](docs/MEDIAN-BENCHMARK-RESULTS.md), and [measurement method](docs/MEDIAN-BENCHMARK-METHOD.md).

### Standalone HTTP server

The 64-byte [`server.rvb`](examples/server.rvb) defines its values, JSON object, port, route, and response status in binary:

```sh
./target/release/rivet build examples/server.rvb -o ./server
./server
```

In another terminal:

```sh
curl -i http://127.0.0.1:8787/hello
```

It returns status `200`, content type `application/json; charset=utf-8`, and `{"message":"hello world"}`. Ctrl-C stops the independent server. The GUI can set port `0` to select an available port.

[`computed.rvb`](examples/computed.rvb), at 159 bytes, exercises checked arithmetic, conversion, concatenation, booleans, and nested JSON. Its `/computed` route returns the computed result. JSON is an output data type here, not the program source.

![Binary source in the workbench and native Hello output](docs/evidence/binary-hello.png)

## Language design

Rivet has two versioned binary forms. RVB1 is a typed, single-assignment instruction stream representing an acyclic dataflow graph. Each instruction's position defines a numeric register. Operands refer to earlier registers; no source names or textual opcodes are stored.

RVB1's four types are `Text`, signed 64-bit `Int`, `Bool`, and `Json`. Seven numeric operations create literals, concatenate text, add integers with overflow checks, convert scalar values to text, and construct objects. An entry record selects console output or an HTTP service. Instructions are evaluated in order, including unused instructions that can fail.

RVB2 adds a computation entry with declared mutable registers, `IntArray`, `IntMap`, `IntList` and finite binary64 `Float`, integer input, comparisons, checked arithmetic, copies, assertions, jumps and conditional branches. Its validator checks that each register has been assigned on every path reaching a read, including across loops. Programs return an integer array, linked list or floating-point scalar. Linked lists have immutable nodes and shared tails; maps are mutable and copied handles preserve aliasing. Existing RVB1 files keep their original encoding and behavior.

The format makes structure explicit and compact. Fixed-width numeric references and length-delimited records support mechanical construction, inspection, and edits. The decoder checks the version, lengths, types, references, UTF-8, canonical field ordering, and resource bounds before native compilation. There is no intermediate JSON source document, embedded text program, or required assembler.

The representation is intended for AI authorship. Its numeric structure alone does not establish better model accuracy or token efficiency; those require separate experiments. See [language semantics](docs/LANGUAGE.md) and [architecture](docs/ARCHITECTURE.md).

## Compilation and runtime

```text
Binary .rvb source → direct decoding and validation → typed IR → LLVM → native executable
```

Rust implements the decoder, validator, compiler driver, and workbench. LLVM/Clang performs optimization and machine-code generation. Generated code implements program operations, control flow and entry configuration; linked C helpers supply text/JSON handling, arrays/maps/list nodes, bounded stdin, allocation, output, and HTTP transport. Native executables contain no Rivet source parser or instruction interpreter.

A reference interpreter executes the validated typed IR for correctness comparisons. The original JSON frontend is retained only as a legacy library/test fixture path; CLI Build/Run/Inspect and the GUI accept binary source and reject JSON source. JSON control messages and HTTP responses are separate from program files.

## Command-line reference

Use `./target/release/rivet` from the checkout, or place the binary on PATH. All source commands consume `.rvb` bytes.

| Command | Behavior |
| --- | --- |
| `rivet build FILE -o PATH [--profile size\|baseline\|speed]` | Compile binary source to a native executable and inspection files. |
| `rivet run FILE` | Build a persistent native artifact and launch it. |
| `rivet inspect FILE` | Decode and validate binary source, then print a readable view. |
| `rivet check FILE` | Validate source and print typed IR; equivalent inspection path. |
| `rivet interpret FILE` | Decode binary source and execute the reference interpreter. |
| `rivet ui [--port PORT]` | Start the local workbench; default port 7331. |

## Development and validation

From the repository root, with Rust, Clang, and Python 3:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
python3 tests/native_runtime/run.py
python3 tests/native_runtime/run.py --sanitize undefined
```

Browser tests require Node.js 22 or newer:

```sh
npm ci
npx playwright install chromium
npm test
```

The workbench itself needs no Node.js or frontend build. Binary tests cover malformed/truncated inputs, canonical roundtrips, types, control flow and definite assignment, decoded offsets, native/reference agreement, binary import/export/editing, and independent executables. Two Sum tests include a seeded oracle corpus and 10,000 elements. Add Two Numbers tests use an independent arbitrary-precision oracle, 100-digit inputs and generic linked-data/aliasing/arithmetic cases. Both use baseline and size compiler profiles. Median adds a sorted-union oracle, exhaustive small inputs and generic floating-point checks under all three profiles. See [Median validation](docs/MEDIAN-VALIDATION.md), [Add Two Numbers validation](docs/ADD-TWO-NUMBERS-VALIDATION.md), [Two Sum validation](docs/TWO-SUM-VALIDATION.md), and the earlier [RVB1 validation](docs/BINARY-VALIDATION.md).

Native profiles offer explicit tradeoffs: default `size` uses Clang `-Oz` plus linker dead stripping, `baseline` uses identical generated LLVM and support at `-O0`, and `speed` uses `-O3` plus dead stripping. All preserve semantic/runtime checks. Run `python3 scripts/median_benchmark.py` after building the release compiler for median runtime and peak-memory comparisons with C and Python; see the [measurement method](docs/MEDIAN-BENCHMARK-METHOD.md). Run `python3 scripts/native_benchmark.py` for the original RVB1 file-size comparison. [Earlier RVB1 measurements](docs/BINARY-BENCHMARK.json) remain historical and do not establish current sizes or broader language efficiency.

The [native fuel optimization](docs/NATIVE-OPTIMIZATION-RESULTS.md) reduced balanced 1000 + 1000 median search from **559.45 to 104.14 ns/query** on an Apple M1 Pro, a **5.37× improvement** in the same warmed preloaded benchmark. Fresh C and Python controls measured 34.47 and 1443.81 ns/query: C's advantage narrowed from about 16× to **3.02×**. The compiler makes the existing instruction-budget check visible to LLVM; all safety limits and failure ordering remain. All eight measured input families improved by 4.56×–5.57×. Whole-process peak RSS for the balanced Rivet case stayed at 1.406 MiB; the executable grew by 16 bytes and median native build time rose by 22 ms. Search timing, process latency and RSS are separate scopes. [Original measurements](docs/MEDIAN-BENCHMARK-RESULTS.md) remain historical evidence. These results do not establish performance across other algorithms or language-wide superiority.

## Scope

RVB2 supports branches and loops within one computation; it does not yet support user-defined functions, recursion, persistent storage, or HTTP request-dependent values. RVB1 HTTP responses are computed at startup. Source is limited to 64 KiB and 256 instructions. RVB2 additionally bounds execution to one million instructions, arrays/maps/list chains to 10,000 elements/entries/nodes, runtime input to 256 KiB and logical cumulative allocations to 64 MiB. [Exact resource bounds](docs/LANGUAGE.md#resource-limits) are documented separately.

HTTP is a local experimental subset: IPv4 loopback, exact GET routes, sequential connections, bounded parsing, and a two-second read deadline. TLS, chunked bodies, streaming, and production deployment are outside this prototype. Only macOS Apple Silicon is verified; cross-compilation, universal binaries, and distribution signing are not provided. Native machine-code emission uses LLVM/Clang; RVB is Rivet's custom binary source format.

## Documentation

- [Binary format and authoring](docs/BINARY-FORMAT.md)
- [Computation format (RVB2)](docs/COMPUTE-FORMAT.md)
- [Two Sum algorithm](docs/TWO-SUM.md)
- [Add Two Numbers algorithm](docs/ADD-TWO-NUMBERS.md)
- [Median algorithm](docs/MEDIAN.md)
- [Comparative benchmark method](docs/MEDIAN-BENCHMARK-METHOD.md)
- [Measured runtime and memory results](docs/MEDIAN-BENCHMARK-RESULTS.md)
- [Native optimization: before/after results](docs/NATIVE-OPTIMIZATION-RESULTS.md)
- [Capability and performance roadmap](docs/ROADMAP.md)
- [Language semantics](docs/LANGUAGE.md)
- [Architecture and runtime boundary](docs/ARCHITECTURE.md)
- [Binary-source validation](docs/BINARY-VALIDATION.md)
- [Project state and history](docs/STATE.md)
