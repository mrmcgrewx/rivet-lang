#!/usr/bin/env python3
"""Build and exercise the native ABI and bounded loopback transport, without Rust."""
import argparse
import contextlib
import json
import os
from pathlib import Path
import re
import selectors
import signal
import socket
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
EXPECTED = {"min": -(2**63), "max": 2**63 - 1, "text": "hi\0\n🙂", "z": True}


def compile_program(source, output, flags, link_runtime=False):
    command = ["clang", "-std=c11", "-Wall", "-Wextra", "-Wpedantic", "-Wconversion",
               "-Wshadow", "-Werror", *flags, str(HERE / source)]
    if link_runtime:
        command.append(str(ROOT / "native/runtime.c"))
    subprocess.run([*command, "-o", str(output)], check=True)


def values_checks(executable):
    subprocess.run([executable], check=True, timeout=10)
    for mode, message in [("concat", "computed Text exceeds 65536 bytes (65537 bytes required)"),
                          ("output", "console output including newline exceeds 65536 bytes (65537 bytes required)"),
                          ("json", "computed Json exceeds 65536 bytes (65537 bytes required)"),
                          ("escaped-json", "computed Json exceeds 65536 bytes (393224 bytes required)")]:
        result = subprocess.run([executable, mode], capture_output=True, timeout=10)
        assert result.returncode == 1, (mode, result.returncode, result.stderr)
        assert message.encode() in result.stderr, (mode, result.stderr)
        assert not result.stdout, mode
    assert subprocess.check_output([executable, "nul-output"], timeout=10) == b"a\0b\n"
    assert subprocess.check_output([executable, "output-ok"], timeout=10) == b"x" * 65535 + b"\n"
    # Console writes to a closed pipe must report an I/O failure, not die of SIGPIPE.
    read_end, write_end = os.pipe()
    os.close(read_end)
    try:
        result = subprocess.run([executable, "nul-output"], stdout=write_end, stderr=subprocess.PIPE, timeout=10)
    finally:
        os.close(write_end)
    assert result.returncode == 1 and b"cannot write console output" in result.stderr


def compute_checks(executable):
    subprocess.run([executable], check=True, timeout=10)

    def success(mode, data, expected):
        result = subprocess.run([executable, mode], input=data, capture_output=True, timeout=10)
        assert result.returncode == 0 and result.stdout == expected and not result.stderr, result

    def failure(mode, data, message):
        result = subprocess.run([executable, mode], input=data, capture_output=True, timeout=10)
        assert result.returncode == 1 and not result.stdout and message.encode() in result.stderr, result

    success("empty", b" \t\r\n\v\f", b"[]\n")
    for value in [-(2**63), 2**63 - 1, -1, 0, 1]:
        success("read-int", str(value).encode(), f"[{value}]\n".encode())
    success("read-int", b"-0000", b"[0]\n")
    success("read-int", b" \t0000123\r\n\v\f", b"[123]\n")
    success("read-array", b"0", b"[]\n")
    success("read-array", b"3 -9223372036854775808 0 9223372036854775807", b"[-9223372036854775808,0,9223372036854775807]\n")
    success("array-then-int", b"4 2 7 11 15 9", b"[4,9]\n")
    # Output and input both exceed the original console capture size.
    large = [-(2**63)] * 10000
    success("read-array", ("10000 " + " ".join(map(str, large))).encode(),
            (json.dumps(large, separators=(",", ":")) + "\n").encode())
    success("read-int", b" " * 262143 + b"1", b"[1]\n")
    success("quota-array-ok", b"", b"")
    success("fuel-getter-last", b"", b"")
    shared_fuel = subprocess.run([executable, "fuel-getter"], input=b"", capture_output=True, timeout=10)
    assert shared_fuel.returncode == 1 and not shared_fuel.stdout
    assert shared_fuel.stderr == b"Rivet runtime: compute instruction limit exceeded (1000000)\n"
    for data in [b"", b" \t\r\n"]:
        failure("read-int", data, "missing an integer")
    for data in [b"+1", b"-", b"1x", b"1.0", b"0x12", b"1\0", b"\xff", "\u00a01".encode()]:
        failure("read-int", data, "invalid signed integer")
    for data in [b"9223372036854775808", b"-9223372036854775809", b"9" * 10000]:
        failure("read-int", data, "signed i64 range")
    failure("read-int", b"1 2", "unconsumed values")
    failure("empty", b"1", "unconsumed values")
    failure("read-int", b" " * 262144 + b"1", "input exceeds 262144 bytes")
    failure("read-array", b"2 1", "missing an integer")
    failure("read-array", b"-1", "must be nonnegative")
    failure("read-array", b"10001", "array length exceeds 10000")
    for mode, message in [("array-limit", "array length exceeds 10000"),
                          ("index-negative", "index out of bounds"),
                          ("index-upper", "index out of bounds"),
                          ("map-limit", "map exceeds 10000 unique keys"),
                          ("quota-map", "allocation budget exceeds 67108864 bytes"),
                          ("quota-array", "allocation budget exceeds 67108864 bytes"),
                          ("fuel", "instruction limit exceeded (1000000)"),
                          ("wrong-type", "invalid native value type")]:
        failure(mode, b"", message)
    read_end, write_end = os.pipe()
    os.close(read_end)
    try:
        result = subprocess.run([executable, "empty"], input=b"", stdout=write_end,
                                stderr=subprocess.PIPE, timeout=10)
    finally:
        os.close(write_end)
    assert result.returncode == 1 and b"cannot write compute output" in result.stderr


def list_checks(executable):
    subprocess.run([executable], check=True, timeout=10)

    def success(mode, data, expected):
        result = subprocess.run([executable, mode], input=data, capture_output=True, timeout=10)
        assert result.returncode == 0 and result.stdout == expected and not result.stderr, result

    def failure(mode, data, message):
        result = subprocess.run([executable, mode], input=data, capture_output=True, timeout=10)
        assert result.returncode == 1 and not result.stdout and message.encode() in result.stderr, result

    success("empty", b" \t\r\n\v\f", b"[]\n")
    success("read", b"-0", b"[]\n")
    success("read", b"1 -0000", b"[0]\n")
    success("read", b"3 -9223372036854775808 17 9223372036854775807",
            b"[-9223372036854775808,17,9223372036854775807]\n")
    success("read", b"3\t-7\r500\v-9\f", b"[-7,500,-9]\n")
    success("two-lists", b"3 2 4 3 3 5 6 4", b"[3,3,2,5]\n")
    success("shared-output", b"", b"[4,2,3]\n")
    success("read", b" " * 262143 + b"0", b"[]\n")
    large = [-(2**63)] * 10000
    success("read", ("10000 " + " ".join(map(str, large))).encode(),
            (json.dumps(large, separators=(",", ":")) + "\n").encode())
    success("quota-ok", b"", b"")
    success("read-exhausted-budget", b"0", b"[]\n")
    for mode in ["nil-value", "nil-next"]:
        failure(mode, b"", "nil list")
    failure("wrong-type", b"", "invalid native value type")
    failure("length-limit", b"", "list length exceeds 10000")
    failure("length-before-budget", b"", "list length exceeds 10000")
    failure("quota-exceeded", b"", "allocation budget exceeds 67108864 bytes")
    failure("read-exhausted-budget", b"1 bad", "allocation budget exceeds 67108864 bytes")
    failure("read-exhausted-budget", b"10001", "list length exceeds 10000")
    failure("read", b"", "missing an integer")
    failure("read", b"2 1", "missing an integer")
    failure("read", b"-1", "must be nonnegative")
    failure("read", b"10001", "list length exceeds 10000")
    failure("read", b"2 7 nope", "invalid signed integer")
    failure("read", b"1 +7", "invalid signed integer")
    failure("read", b"1 9223372036854775808", "signed i64 range")
    failure("read", b"1 5 7", "unconsumed values")
    failure("empty", b"1", "unconsumed values")
    failure("read", b" " * 262144 + b"0", "input exceeds 262144 bytes")
    read_end, write_end = os.pipe()
    os.close(read_end)
    try:
        result = subprocess.run([executable, "empty"], input=b"", stdout=write_end,
                                stderr=subprocess.PIPE, timeout=10)
    finally:
        os.close(write_end)
    assert result.returncode == 1 and b"cannot write compute output" in result.stderr


def float_checks(executable):
    for mode, expected in [("half", b"2.5000000000000000e+00\n"),
                           ("negative-zero", b"-0.0000000000000000e+00\n"),
                           ("max", b"1.7976931348623157e+308\n"),
                           ("subnormal", b"4.9406564584124654e-324\n")]:
        result = subprocess.run([executable, mode], input=b" \t\n", capture_output=True, timeout=10)
        assert result.returncode == 0 and result.stdout == expected and not result.stderr, result
    for mode, data, message in [("infinity", b"", b"nonfinite Float output"),
                                ("nan", b"", b"nonfinite Float output"),
                                ("half", b"1", b"unconsumed values"),
                                ("half", b" " * 262145, b"input exceeds 262144 bytes")]:
        result = subprocess.run([executable, mode], input=data, capture_output=True, timeout=10)
        assert result.returncode == 1 and not result.stdout and message in result.stderr, result
    read_end, write_end = os.pipe()
    os.close(read_end)
    try:
        result = subprocess.run([executable, "half"], input=b"", stdout=write_end,
                                stderr=subprocess.PIPE, timeout=10)
    finally:
        os.close(write_end)
    assert result.returncode == 1 and b"cannot write compute output" in result.stderr


@contextlib.contextmanager
def server(executable, port=0, control=None):
    env = os.environ.copy()
    env.pop("RIVET_CONTROL", None)
    if control is not None:
        env["RIVET_CONTROL"] = control
    process = subprocess.Popen([executable, str(port)], stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, env=env)
    try:
        with selectors.DefaultSelector() as selector:
            selector.register(process.stdout, selectors.EVENT_READ)
            assert selector.select(5), "server did not report readiness"
        ready = process.stdout.readline()
        match = re.fullmatch(rb"Rivet program server: http://127\.0\.0\.1:(\d+)\n", ready)
        assert match, (ready, process.poll())
        yield process, int(match.group(1))
    finally:
        if process.poll() is None:
            process.send_signal(signal.SIGTERM)
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
            raise AssertionError("server did not stop within two seconds")
        process.stdout.close()
        process.stderr.close()


def request(port, raw):
    with socket.create_connection(("127.0.0.1", port), timeout=4) as connection:
        connection.sendall(raw)
        result = bytearray()
        while True:
            try:
                part = connection.recv(8192)
            except ConnectionResetError:
                # Rejecting an oversized frame closes without draining input.
                break
            if not part:
                break
            result.extend(part)
    headers, body = bytes(result).split(b"\r\n\r\n", 1)
    lines = headers.split(b"\r\n")
    status = int(lines[0].split()[1])
    parsed = {key.lower(): value.strip() for key, value in
              (line.split(b":", 1) for line in lines[1:])}
    assert parsed[b"content-type"] == b"application/json; charset=utf-8"
    assert int(parsed[b"content-length"]) == len(body)
    return status, parsed, body


def get(port, path="/hello"):
    return request(port, f"GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n".encode())


def http_checks(executable):
    with server(executable, control="1") as (process, port):
        for _ in range(8):
            status, _, body = get(port, "/hello?query=yes")
            assert status == 201 and json.loads(body) == EXPECTED
            assert body == json.dumps(EXPECTED, ensure_ascii=False, sort_keys=True,
                                      separators=(",", ":")).encode()
        assert get(port, "/missing")[0] == 404
        status, headers, _ = request(port, b"POST /hello HTTP/1.1\r\nContent-Length: 0\r\n\r\n")
        assert status == 405 and headers[b"allow"] == b"GET"
        assert get(port, "/" + "x" * 2047)[0] == 200
        assert get(port, "/" + "x" * 2048)[0] == 400
        status, _, body = get(port, "/large")
        assert status == 200 and len(body) == 65536
        invalid = [
            (b"Content-Length: 100000000000000000000000000000000\r\n", 413),
            (b"Content-Length: 524289\r\n", 413),
            (b"Content-Length: 0\r\nContent-Length: 0\r\n", 400),
            (b"Content-Length: -1\r\n", 400),
            (b"Transfer-Encoding: chunked\r\n", 501),
            (b"Expect: 100-continue\r\n", 417),
            (b"Host: x\r\nHost: y\r\n", 400),
            (b"Bad Header: x\r\n", 400),
            (b"X: \0\r\n", 400),
            (b"X: x\r\n" * 65, 431),
            (b"X: " + b"x" * 8192 + b"\r\n", 431),
        ]
        for headers, expected in invalid:
            assert request(port, b"GET /hello HTTP/1.1\r\n" + headers + b"\r\n")[0] == expected
        status, _, body = request(port, b"GET /hello HTTP/1.1\r\nContent-Length: 524288\r\n\r\n" + b"x" * 524288)
        assert status == 201 and json.loads(body) == EXPECTED
        started = time.monotonic()
        assert request(port, b"GET /hello HTTP/1.1\r\nContent-Length: 3\r\n\r\nx")[0] == 408
        assert 1.8 < time.monotonic() - started < 3.5
        # An abandoned response must not kill the listener with SIGPIPE.
        for _ in range(5):
            connection = socket.create_connection(("127.0.0.1", port))
            connection.sendall(b"GET /large HTTP/1.1\r\n\r\n")
            connection.close()
        assert get(port)[0] == 201
        # A second process cannot bind the existing loopback listener.
        conflict = subprocess.run([executable, str(port)], capture_output=True, timeout=2)
        assert conflict.returncode == 1 and b"cannot bind HTTP listener" in conflict.stderr
        process.send_signal(signal.SIGINT)
        assert process.wait(timeout=1) == 0
        logs = process.stderr.read().splitlines()
        assert len(logs) >= 27 and set(logs) == {b"RIVET_REQUEST"}, logs
    with server(executable, port=port) as (replacement, _):
        assert get(port)[0] == 201
        replacement.send_signal(signal.SIGTERM)
        assert replacement.wait(timeout=1) == 0
        assert replacement.stderr.read() == b""

    for incomplete in [b"GET /hello HTTP/1.1\r\nHost:",
                       b"GET /hello HTTP/1.1\r\nContent-Length: 2048\r\n\r\nx"]:
        with server(executable) as (process, port):
            with socket.create_connection(("127.0.0.1", port)) as connection:
                connection.sendall(incomplete)
                time.sleep(0.05)
                started = time.monotonic()
                process.send_signal(signal.SIGTERM)
                assert process.wait(timeout=1) == 0
                assert time.monotonic() - started < 0.5
                connection.settimeout(1)
                assert connection.recv(1) == b""
        with server(executable, port=port):
            assert get(port)[0] == 201


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--sanitize", nargs="?", const="address,undefined")
    args = parser.parse_args()
    profiles = [("baseline", ["-O0"]), ("size", ["-Oz"]), ("speed", ["-O3"])]
    if args.sanitize:
        profiles = [(args.sanitize.replace(",", "-"), ["-O1", "-g", f"-fsanitize={args.sanitize}",
                                    "-fno-omit-frame-pointer", "-fno-sanitize-recover=all"])]
    with tempfile.TemporaryDirectory(prefix="build-", dir=HERE) as build:
        for name, flags in profiles:
            values = str(Path(build) / f"values-{name}")
            http = str(Path(build) / f"server-{name}")
            compute = str(Path(build) / f"compute-{name}")
            lists = str(Path(build) / f"lists-{name}")
            floats = str(Path(build) / f"floats-{name}")
            compile_program("values.c", values, flags)
            compile_program("server.c", http, flags, link_runtime=True)
            compile_program("compute.c", compute, flags)
            compile_program("lists.c", lists, flags)
            compile_program("floats.c", floats, flags)
            values_checks(values)
            compute_checks(compute)
            list_checks(lists)
            float_checks(floats)
            http_checks(http)
            print(f"{name}: native values, compute input/arrays/maps/lists/floats/limits, HTTP framing, shutdown and rebind passed", flush=True)


if __name__ == "__main__":
    main()
