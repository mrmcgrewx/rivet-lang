"""Frozen task data, independent oracles and output checks for the authoring pilot."""
import collections
import hashlib
import itertools
import json
import math
from pathlib import Path
import random

BASE = Path(__file__).resolve().parent
PROJECT = BASE.parents[1]
TASKS = {"two_sum": 1, "add_two_numbers": 2, "median": 3}
LANGUAGES = ("rivet", "c", "rust")
MASK = (1 << 64) - 1


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def expected(task, case):
    a, b = case["a"], case.get("b", [])
    if task == "two_sum":
        target = case["target"]
        if len(a) <= 128:
            pairs = [(i, j) for i in range(len(a)) for j in range(i + 1, len(a))
                     if a[i] + a[j] == target]
            assert len(pairs) == 1
            return list(pairs[0])
        # Count unordered value-pairs independently of a streaming map solver.
        counts = collections.Counter(a)
        hits = []
        number = 0
        for x, n in counts.items():
            y = target - x
            if x < y:
                found = n * counts[y]
            elif x == y:
                found = n * (n - 1) // 2
            else:
                found = 0
            if found:
                number += found
                hits = [i for i, value in enumerate(a) if value == x or value == y]
        assert number == 1 and len(hits) == 2
        return sorted(hits)
    if task == "add_two_numbers":
        def integer(digits):
            return int("".join(map(str, reversed(digits))))
        return [int(x) for x in reversed(str(integer(a) + integer(b)))]
    merged = sorted(a + b)
    middle = len(merged) // 2
    assert merged
    return float(merged[middle]) if len(merged) % 2 else (merged[middle - 1] + merged[middle]) / 2


def encode(task, case):
    tokens = [len(case["a"]), *case["a"]]
    if task == "two_sum":
        tokens.append(case["target"])
    else:
        tokens += [len(case["b"]), *case["b"]]
    return (" ".join(map(str, tokens)) + "\n").encode()


def normalized(task, value):
    if task == "median":
        if isinstance(value, bool) or not isinstance(value, (float, int)) or not math.isfinite(value):
            raise ValueError("expected a finite numeric median")
        return float(value)
    if not isinstance(value, list) or any(type(x) is not int for x in value):
        raise ValueError("expected a JSON integer array")
    if task == "two_sum":
        if len(value) != 2 or value[0] == value[1]:
            raise ValueError("expected two distinct indices")
        return sorted(value)
    if not 1 <= len(value) <= 101 or any(not 0 <= x <= 9 for x in value):
        raise ValueError("expected 1..101 reversed decimal digits")
    return value


def check_output(task, case, output):
    try:
        result = normalized(task, json.loads(output))
    except (ValueError, TypeError) as exc:
        return {"ok": False, "error": str(exc), "actual": output[:400]}
    wanted = expected(task, case)
    return {"ok": result == wanted, "expected": wanted, "actual": result}


def checksum(values):
    result = 1469598103934665603
    for value in [len(values), *values]:
        result = ((result ^ (value & MASK)) * 1099511628211) & MASK
    return result


def pool_text(task, cases):
    lines = [str(len(cases))]
    for case in cases:
        value = expected(task, case)
        values = [int(value * 2)] if task == "median" else value
        lines += [encode(task, case).decode().strip(), " ".join(map(str, [len(values), *values]))]
    return "\n".join(lines) + "\n"


def pool_checksum(task, cases):
    result = 0
    for i, case in enumerate(cases):
        value = expected(task, case)
        values = [int(value * 2)] if task == "median" else value
        result = (result + checksum(values) * (1 + i % 17)) & MASK
    return result


PUBLIC = {
    "two_sum": [{"a": [2, 7, 11, 15], "target": 9}, {"a": [3, 2, 4], "target": 6},
                {"a": [3, 3], "target": 6}],
    "add_two_numbers": [{"a": [2, 4, 3], "b": [5, 6, 4]}, {"a": [0], "b": [0]},
                        {"a": [9] * 7, "b": [9] * 4}],
    "median": [{"a": [1, 3], "b": [2]}, {"a": [1, 2], "b": [3, 4]}],
}


def two_case(rng, n):
    # Exactly one odd value; target selects exactly one distinct even partner.
    evens = [2 * x for x in rng.sample(range(-400000000, 400000001), n - 1)]
    odd = rng.randrange(-100000000, 100000001) * 2 + 1
    partner = rng.choice(evens)
    target = odd + partner
    assert -10**9 <= target <= 10**9
    values = evens + [odd]
    rng.shuffle(values)
    return {"a": values, "target": target}


def digits(rng, count):
    values = [rng.randrange(10) for _ in range(count)]
    if count > 1:
        values[-1] = rng.randrange(1, 10)
    return values


def generate_corpora():
    dev_rng, hidden_rng, bench_rng = (random.Random(x) for x in (84173, 2026102601, 2026102602))
    dev = {task: [dict(case) for case in cases] for task, cases in PUBLIC.items()}
    dev["two_sum"] += [{"a": [0, 4, 3, 0], "target": 0}, {"a": [-3, 4, 3, 90], "target": 0}]
    dev["two_sum"] += [two_case(dev_rng, n) for n in (2, 8, 31, 1000, 10000)]
    dev["add_two_numbers"] += [{"a": [9] * 100, "b": [1]}, {"a": [0], "b": [4, 5, 6]}]
    dev["add_two_numbers"] += [{"a": digits(dev_rng, n), "b": digits(dev_rng, m)}
                               for n, m in ((1, 100), (32, 25), (100, 100), (17, 2), (1, 1))]
    dev["median"] += [{"a": [], "b": [0]}, {"a": [-5, -3], "b": [-4, -2]},
                      {"a": [2, 2], "b": [2]}, {"a": [1000000], "b": [-1000000]}]
    dev["median"] += [{"a": sorted(dev_rng.randint(-10**6, 10**6) for _ in range(n)),
                       "b": sorted(dev_rng.randint(-10**6, 10**6) for _ in range(m))}
                      for n, m in ((0, 1000), (1000, 0), (1000, 1000), (1, 1000))]
    holdout = {task: [] for task in TASKS}
    for n in (2, 3, 4, 8, 16, 31, 64, 100, 127, 1000, 10000):
        holdout["two_sum"] += [two_case(hidden_rng, n) for _ in range(16)]
    holdout["two_sum"] += [{"a": [x, x], "target": 2 * x} for x in (-500000000, -1, 0, 1, 500000000)]
    # Random duplicate-rich small arrays with independently established unique pair.
    while len(holdout["two_sum"]) < 256:
        case = {"a": [hidden_rng.randint(-20, 20) for _ in range(hidden_rng.randint(2, 12))],
                "target": hidden_rng.randint(-40, 40)}
        try:
            expected("two_sum", case)
        except AssertionError:
            continue
        holdout["two_sum"].append(case)
    for _ in range(256):
        holdout["add_two_numbers"].append({"a": digits(hidden_rng, hidden_rng.randint(1, 100)),
                                           "b": digits(hidden_rng, hidden_rng.randint(1, 100))})
    holdout["add_two_numbers"] += [{"a": [9] * n, "b": [1]} for n in (1, 2, 31, 99, 100)]
    holdout["add_two_numbers"] += [{"a": [0], "b": [0]}, {"a": [1], "b": [9] * 100},
                                   {"a": [0] * 99 + [1], "b": [0] * 99 + [9]}]
    small = [list(x) for n in range(3) for x in itertools.combinations_with_replacement((-1, 0, 1), n)]
    holdout["median"] += [{"a": x, "b": y} for x in small for y in small if x or y]
    for i in range(256):
        n, m = hidden_rng.randrange(1001), hidden_rng.randrange(1001)
        if n + m == 0:
            m = 1
        span = 3 if i % 4 == 0 else 1000000
        holdout["median"].append({"a": sorted(hidden_rng.randint(-span, span) for _ in range(n)),
                                  "b": sorted(hidden_rng.randint(-span, span) for _ in range(m))})
    holdout["median"] += [{"a": [], "b": [-1000000, 1000000]},
                          {"a": [-1000000] * 1000, "b": [1000000] * 1000},
                          {"a": list(range(0, 2000, 2)), "b": list(range(1, 2001, 2))}]
    pools = {task: {} for task in TASKS}
    for label, n in (("small_8", 8), ("medium_1000", 1000), ("large_10000", 10000)):
        pools["two_sum"][label] = [two_case(bench_rng, n) for _ in range(8)]
    for label, n in (("small_1", 1), ("medium_32", 32), ("large_100", 100)):
        pools["add_two_numbers"][label] = [{"a": digits(bench_rng, n), "b": digits(bench_rng, n)} for _ in range(8)]
    for label, n, m in (("small_4", 4, 4), ("medium_100", 100, 100), ("large_1000", 1000, 1000), ("asymmetric", 1, 1000)):
        pools["median"][label] = [{"a": sorted(bench_rng.randint(-10**6, 10**6) for _ in range(n)),
                                   "b": sorted(bench_rng.randint(-10**6, 10**6) for _ in range(m))} for _ in range(8)]
    for task in TASKS:
        for case in [*dev[task], *holdout[task], *(c for rows in pools[task].values() for c in rows)]:
            expected(task, case)
    return {"format": "rivet-agent-corpora-v1", "seeds": [84173, 2026102601, 2026102602],
            "development": dev, "holdout": holdout, "performance": pools}


if __name__ == "__main__":
    (BASE / "corpora.json").write_text(json.dumps(generate_corpora(), separators=(",", ":")) + "\n")
    print("Wrote deterministic corpora.json")
