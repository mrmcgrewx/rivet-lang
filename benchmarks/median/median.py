#!/usr/bin/env python3
"""Same partition algorithm, plus a separately timed preloaded-data harness."""
import sys

MIN_I64 = -(2**63)
MAX_I64 = 2**63 - 1


def median_partition(a, b):
    m, n = len(a), len(b)
    if m > 1000 or n > 1000 or m + n == 0:
        raise ValueError("invalid lengths")
    if m > n:
        a, b, m, n = b, a, n, m
    low, high = 0, m
    half = (m + n + 1) // 2
    while low <= high:
        i = (low + high) // 2
        j = half - i
        left_a = MIN_I64 if i == 0 else a[i - 1]
        right_a = MAX_I64 if i == m else a[i]
        left_b = MIN_I64 if j == 0 else b[j - 1]
        right_b = MAX_I64 if j == n else b[j]
        if left_a <= right_b and left_b <= right_a:
            left = max(left_a, left_b)
            if (m + n) % 2:
                return float(left)
            right = min(right_a, right_b)
            return (float(left) + float(right)) / 2.0
        if left_a > right_b:
            high = i - 1
        else:
            low = i + 1
    raise ValueError("no partition")


def pair(tokens):
    m = int(next(tokens))
    if not 0 <= m <= 1000:
        raise ValueError("invalid length")
    a = [int(next(tokens)) for _ in range(m)]
    n = int(next(tokens))
    if not 0 <= n <= 1000:
        raise ValueError("invalid length")
    return a, [int(next(tokens)) for _ in range(n)]


def evaluate(case):
    return median_partition(case[0], case[1])


def control(case):
    return case[2]


def preloaded(path, rounds, warmups, mode):
    # Benchmark-only imports must not inflate the standalone startup comparison.
    import json
    import math
    import time
    with open(path) as dataset:
        tokens = iter(dataset.read().split())
    count = int(next(tokens))
    if (not 1 <= count <= 256 or not 1 <= rounds <= 100_000_000 // count
            or not 0 <= warmups <= 100_000_000 // count
            or mode not in ("evaluate", "control")):
        raise ValueError("benchmark size outside bounds")
    cases = []
    for _ in range(count):
        a, b = pair(tokens)
        expected = float(next(tokens))
        cases.append((a, b, expected))
    if next(tokens, None) is not None:
        raise ValueError("extra benchmark data")
    del tokens
    expected_checksum = 0.0
    for index, case in enumerate(cases):
        result = evaluate(case)
        if not math.isfinite(result) or result != case[2]:
            raise ValueError("oracle mismatch")
        expected_checksum += result * (1 + index % 17)
    invoke = evaluate if mode == "evaluate" else control
    for _ in range(warmups):
        for case in cases:
            invoke(case)
    checksum = 0.0
    started = time.perf_counter_ns()
    for _ in range(rounds):
        for index, case in enumerate(cases):
            checksum += invoke(case) * (1 + index % 17)
    elapsed = time.perf_counter_ns() - started
    if checksum != expected_checksum * rounds:
        raise ValueError("checksum mismatch")
    print(json.dumps({"implementation": "python", "mode": mode, "cases": count,
                      "rounds": rounds, "calls": count * rounds, "warmup_calls": count * warmups,
                      "elapsed_ns": elapsed, "checksum": checksum,
                      "expected_checksum_per_round": expected_checksum}, separators=(",", ":")))


def main():
    if len(sys.argv) == 6 and sys.argv[1] == "--preloaded":
        preloaded(sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), sys.argv[5])
        return
    if len(sys.argv) != 1:
        raise ValueError("unexpected arguments")
    data = sys.stdin.buffer.read(262145)
    if len(data) > 262144:
        raise ValueError("input too large")
    tokens = iter(data.split())
    a, b = pair(tokens)
    if next(tokens, None) is not None:
        raise ValueError("unconsumed data")
    print(f"{median_partition(a, b):.16e}")


if __name__ == "__main__":
    main()
