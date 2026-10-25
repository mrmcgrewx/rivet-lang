#!/usr/bin/env python3
"""Reproducible median runtime/RSS comparison; see docs/MEDIAN-BENCHMARK-METHOD.md."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import random
import re
import shutil
import signal
import statistics
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
BENCH = ROOT / "benchmarks/median"
SOURCE = ROOT / "examples/median-two-sorted-arrays.rvb"
LANGUAGES = ("rivet", "c", "python")


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def command(args):
    return subprocess.run(list(map(str, args)), check=True, capture_output=True,
                          text=True, timeout=60).stdout.strip()


def oracle(a, b):
    merged = sorted(a + b)  # Independent full-sort oracle, never partition search.
    middle = len(merged) // 2
    return float(merged[middle]) if len(merged) % 2 else (merged[middle - 1] + merged[middle]) / 2


def encode(a, b):
    return (" ".join(map(str, [len(a), *a, len(b), *b])) + "\n").encode()


def corpus(seed, count):
    rng = random.Random(seed)
    groups = {}
    for group in ("tiny", "balanced_1000", "asymmetric", "empty_side", "disjoint", "duplicates", "interleaved", "negative"):
        cases = []
        for index in range(count):
            if group == "tiny":
                m, n = rng.randrange(9), rng.randrange(1, 9)
                a, b = [rng.randrange(-20, 21) for _ in range(m)], [rng.randrange(-20, 21) for _ in range(n)]
                fixed = [([1, 3], [2]), ([1, 2], [3, 4]), ([-5, -3], [-4, -2]), ([0], [0])]
                if index < len(fixed):
                    a, b = fixed[index]
            elif group == "balanced_1000":
                a = [rng.randrange(-1000000, 1000001) for _ in range(1000)]
                b = [rng.randrange(-1000000, 1000001) for _ in range(1000)]
            elif group == "asymmetric":
                a = [rng.randrange(-1000000, 1000001) for _ in range(1 + index % 8)]
                b = [rng.randrange(-1000000, 1000001) for _ in range(1000)]
            elif group == "empty_side":
                a, b = [], [rng.randrange(-1000000, 1000001) for _ in range(1 + (index * 67) % 1000)]
            elif group == "disjoint":
                a = [rng.randrange(-1000000, -1) for _ in range(1000)]
                b = [rng.randrange(1, 1000001) for _ in range(1 + index * 53 % 1000)]
            elif group == "duplicates":
                a = [rng.randrange(-2, 3) for _ in range(1000)]
                b = [rng.randrange(-2, 3) for _ in range(1 + index * 71 % 1000)]
            elif group == "interleaved":
                offset = rng.randrange(-1000, 1001)
                a, b = list(range(offset, offset + 2000, 2)), list(range(offset + 1, offset + 2001, 2))
            else:
                a = [rng.randrange(-1000000, 0) for _ in range(1000)]
                b = [rng.randrange(-1000000, 0) for _ in range(1 + index * 97 % 1000)]
            a.sort(); b.sort()
            if index % 2:
                a, b = b, a  # Both argument orders exercise the shorter-array swap.
            cases.append({"id": f"{group}-{index}", "a": a, "b": b, "expected": oracle(a, b)})
        rng.shuffle(cases)
        groups[group] = cases
    return groups


def run_child(args, data=b"", timeout=30):
    """wait4 returns THIS pid's resource usage, never cumulative RUSAGE_CHILDREN."""
    def timed_out(_signum, _frame):
        raise TimeoutError(f"benchmark child exceeded {timeout}s: {args[0]}")

    env = os.environ.copy()
    env["PYTHONHASHSEED"] = "0"
    with tempfile.TemporaryFile() as incoming, tempfile.TemporaryFile() as outgoing, tempfile.TemporaryFile() as errors:
        incoming.write(data); incoming.seek(0)
        previous_handler = signal.signal(signal.SIGALRM, timed_out)
        child = None
        try:
            signal.setitimer(signal.ITIMER_REAL, timeout)
            started = time.perf_counter_ns()
            # Absolute executable paths + close_fds=False allow CPython's posix_spawn
            # path on the verified host; no cwd/preexec_fn or launcher wrapper.
            child = subprocess.Popen(list(map(str, args)), stdin=incoming, stdout=outgoing,
                                     stderr=errors, env=env, close_fds=False)
            _, status, usage = os.wait4(child.pid, 0)
            elapsed = time.perf_counter_ns() - started
            child.returncode = os.waitstatus_to_exitcode(status)
        except BaseException:
            if child is not None and child.returncode is None:
                child.kill()
                _, status, _ = os.wait4(child.pid, 0)
                child.returncode = os.waitstatus_to_exitcode(status)
            raise
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            signal.signal(signal.SIGALRM, previous_handler)
        outgoing.seek(0); stdout = outgoing.read().decode()
        errors.seek(0); stderr = errors.read().decode()
    if child.returncode != 0 or stderr:
        raise RuntimeError(f"child failed: {args}\nexit={child.returncode}\n{stderr}")
    raw_rss = usage.ru_maxrss
    return {"elapsed_ns": elapsed, "peak_rss_bytes": int(raw_rss if sys.platform == "darwin" else raw_rss * 1024),
            "peak_rss_raw": raw_rss, "user_cpu_seconds": usage.ru_utime,
            "system_cpu_seconds": usage.ru_stime, "stdout": stdout}


def distribution(values):
    ordered = sorted(values)
    def percentile(q):
        offset = (len(ordered) - 1) * q
        low = int(offset)
        high = min(low + 1, len(ordered) - 1)
        return ordered[low] + (ordered[high] - ordered[low]) * (offset - low)
    return {"samples": len(values), "min": min(values), "p10": percentile(.1),
            "median": statistics.median(values), "p90": percentile(.9), "max": max(values)}


def build(args, work):
    clang = Path(shutil.which(os.environ.get("RIVET_CLANG", "clang")) or "clang").resolve()
    flags = ["-std=c11", "-O3", "-Wall", "-Wextra", "-Wpedantic", "-Wconversion", "-Wshadow", "-Werror"]
    strip = "-Wl,-dead_strip" if sys.platform == "darwin" else "-Wl,--gc-sections"
    flags += ["-ffunction-sections", "-fdata-sections"]
    commands = []
    def invoke(values):
        commands.append(list(map(str, values)))
        return command(values)
    rivet = args.rivet.resolve()
    invoke([rivet, "build", SOURCE, "-o", work / "rivet-median", "--profile", "speed"])
    invoke([clang, *flags, BENCH / "partition.c", BENCH / "standalone.c", strip, "-o", work / "c-median"])
    invoke([clang, *flags, BENCH / "partition.c", BENCH / "preloaded.c", strip, "-o", work / "c-preloaded"])
    original = (work / "rivet-median.ll").read_text()
    transformed = original
    substitutions = {"main": "rivet_bench_eval", "rv2_read_array": "bench_read_array",
                     "rv2_return_float": "bench_return_float", "rv_cleanup": "bench_cleanup"}
    counts = {}
    for old, new in substitutions.items():
        counts[old] = transformed.count(f"@{old}(")
        if counts[old] == 0 or f"@{new}(" in transformed:
            raise RuntimeError(f"unexpected LLVM boundary shape: {old}")
        transformed = transformed.replace(f"@{old}(", f"@{new}(")
    restored = transformed
    for old, new in substitutions.items():
        restored = restored.replace(f"@{new}(", f"@{old}(")
    if restored != original or transformed.count("call void @rv2_tick()") != original.count("call void @rv2_tick()"):
        raise RuntimeError("benchmark transformation changed non-boundary code")
    (work / "rivet-preloaded.ll").write_text(transformed)
    invoke([clang, "-O3", "-c", work / "rivet-preloaded.ll", "-o", work / "rivet-preloaded.o"])
    invoke([clang, *flags, "-DBENCH_RIVET", BENCH / "preloaded.c", work / "rivet-preloaded.o", strip,
            "-o", work / "rivet-preloaded"])
    standalone = {"rivet": [work / "rivet-median"], "c": [work / "c-median"],
                  "python": [sys.executable, BENCH / "median.py"]}
    preloaded = {"rivet": [work / "rivet-preloaded"], "c": [work / "c-preloaded"],
                 "python": [sys.executable, BENCH / "median.py", "--preloaded"]}
    return standalone, preloaded, {"commands": commands, "clang": command([clang, "--version"]),
        "substitutions": substitutions, "substitution_occurrences": counts,
        "generated_tick_call_sites": original.count("call void @rv2_tick()"),
        "original_llvm_sha256": hashlib.sha256(original.encode()).hexdigest(),
        "preloaded_llvm_sha256": hashlib.sha256(transformed.encode()).hexdigest(),
        "artifact_bytes": {name: (work / name).stat().st_size for name in ("rivet-median", "c-median", "rivet-preloaded", "c-preloaded")},
        "artifact_sha256": {name: digest(work / name) for name in ("rivet-median", "c-median", "rivet-preloaded", "c-preloaded")}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rivet", type=Path, default=ROOT / "target/release/rivet")
    parser.add_argument("--report", type=Path, default=ROOT / ".rivet/median-benchmark.json")
    parser.add_argument("--seed", type=int, default=20261025)
    parser.add_argument("--samples", type=int, default=7)
    parser.add_argument("--cases-per-group", type=int, default=16)
    parser.add_argument("--target-ms", type=float, default=50)
    parser.add_argument("--warmup-rounds", type=int, default=32)
    args = parser.parse_args()
    if sys.platform not in ("darwin", "linux") or not hasattr(os, "wait4"):
        parser.error("per-child RSS collection currently requires macOS or Linux wait4")
    if not 1 <= args.samples <= 100 or not 1 <= args.cases_per_group <= 256 or not .1 <= args.target_ms <= 1000 or not 0 <= args.warmup_rounds <= 10000:
        parser.error("parameters outside bounded benchmark limits")
    if not args.rivet.is_file():
        parser.error("build the release compiler first: cargo build --release --locked")
    parent = ROOT / ".rivet/benchmarks"
    parent.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="median-", dir=parent))
    standalone, preloaded, build_info = build(args, work)
    groups = corpus(args.seed, args.cases_per_group)
    rng = random.Random(args.seed ^ 0x5EED)
    report = {"format": "rivet-median-benchmark-v1", "recorded_at_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
              "measurement_intent": "functional_smoke" if args.samples < 5 or args.target_ms < 10 or args.cases_per_group < 8 else "comparative_measurement",
              "revision": command(["git", "-C", ROOT, "rev-parse", "HEAD"]),
              "working_tree_status": command(["git", "-C", ROOT, "status", "--short"]),
              "host": {"os": platform.system(), "release": platform.release(), "machine": platform.machine(),
                       "cpu_count": os.cpu_count(), "python": sys.version, "python_implementation": platform.python_implementation()},
              "parameters": {"seed": args.seed, "samples": args.samples, "cases_per_group": args.cases_per_group,
                             "target_ms": args.target_ms, "warmup_rounds": args.warmup_rounds, "maximum_calibrated_calls": 2000000},
              "units": {"elapsed": "nanoseconds", "peak_rss": "bytes", "native_wait4_maxrss": "bytes" if sys.platform == "darwin" else "KiB"},
              "scopes": {"process": "parent monotonic clock around Popen launch through wait4 exit; stdin parsing/output included; temp-file preparation excluded",
                         "preloaded": "child monotonic clock around repeated calls and weighted checksum; setup/parsing/formatting excluded; boundary and safety checks retained",
                         "preloaded_control": "same loop and checksum reading expected results; a harness floor only, never subtracted",
                         "memory": "wait4 peak RSS of each entire child including startup, parsing/preload, warmup and measurement; not auxiliary algorithm storage"},
              "build": build_info, "work_directory": str(work), "cases": groups,
              "verification": [], "orders": [], "raw_samples": [], "summary": {}}
    clock = time.get_clock_info("perf_counter")
    report["clocks"] = {"parent_and_python": {"implementation": clock.implementation,
                                            "resolution_seconds": clock.resolution,
                                            "monotonic": clock.monotonic},
                        "native_preloaded": "clock_gettime(CLOCK_MONOTONIC)"}
    if sys.platform == "darwin":
        report["host"]["cpu"] = command(["sysctl", "-n", "machdep.cpu.brand_string"])
    paths = [SOURCE, ROOT / "native/runtime.c", ROOT / "native/runtime.h", args.rivet,
             Path(__file__), *sorted(BENCH.glob("*.c")), *sorted(BENCH.glob("*.h")), BENCH / "median.py"]
    report["sha256"] = {str(p.relative_to(ROOT)) if p.is_relative_to(ROOT) else str(p): digest(p) for p in paths}
    for group, cases in groups.items():
        text = str(len(cases)) + "\n" + "".join(encode(c["a"], c["b"]).decode().strip() + f" {c['expected']:.17g}\n" for c in cases)
        (work / f"{group}.txt").write_text(text)
        for case in cases:
            outputs = {}
            for language in LANGUAGES:
                result = run_child(standalone[language], encode(case["a"], case["b"]))
                if not re.fullmatch(r"-?\d\.\d{16}e[+-]\d{2,3}\n", result["stdout"]) or float(result["stdout"]) != case["expected"]:
                    raise AssertionError((case["id"], language, result["stdout"]))
                outputs[language] = result["stdout"]
            report["verification"].append({"case": case["id"], "expected": case["expected"], "standalone_outputs": outputs})
    print(f"Verified {sum(map(len, groups.values()))} inputs against all three unchanged standalone programs.", flush=True)

    # Whole-process cases: one fixed representative from each family, warmup and
    # repeated randomized orders. Each record is an independent child's RSS.
    for trial in range(-1, args.samples):
        jobs = [(group, language) for group in groups for language in LANGUAGES]
        rng.shuffle(jobs)
        report["orders"].append({"scope": "process", "trial": trial, "jobs": jobs})
        for group, language in jobs:
            case = groups[group][0]
            result = run_child(standalone[language], encode(case["a"], case["b"]))
            if float(result["stdout"]) != case["expected"]:
                raise AssertionError("process output mismatch")
            report["raw_samples"].append({"scope": "process", "phase": "warmup" if trial < 0 else "measure", "trial": trial,
                                           "group": group, "case": case["id"], "language": language, **result})
    print("Whole-process samples complete; calibrating repeated preloaded calls.", flush=True)

    rounds = {}
    def loaded(group, language, repetitions, mode):
        result = run_child([*preloaded[language], work / f"{group}.txt", repetitions, args.warmup_rounds, mode])
        payload = json.loads(result.pop("stdout"))
        cases = groups[group]
        expected = sum(c["expected"] * (1 + i % 17) for i, c in enumerate(cases))
        if payload["checksum"] != expected * repetitions or payload["calls"] != len(cases) * repetitions or payload["expected_checksum_per_round"] != expected:
            raise AssertionError("preloaded checksum/call count mismatch")
        result["process_elapsed_ns"] = result.pop("elapsed_ns")
        result.update(payload)
        result["ns_per_call"] = payload["elapsed_ns"] / payload["calls"]
        return result
    jobs = [(group, language) for group in groups for language in LANGUAGES]
    rng.shuffle(jobs)
    report["orders"].append({"scope": "calibration", "jobs": jobs})
    for group, language in jobs:
        result = loaded(group, language, 128, "evaluate")
        wanted = math.ceil(args.target_ms * 1000000 / max(result["elapsed_ns"], 1) * 128)
        rounds[(group, language)] = max(1, min(wanted, 2000000 // len(groups[group])))
        report["raw_samples"].append({"scope": "preloaded", "phase": "calibration", "group": group, "language": language, **result})
    report["calibrated_rounds"] = {group: {lang: rounds[group, lang] for lang in LANGUAGES} for group in groups}
    for trial in range(args.samples):
        jobs = [(group, language, mode) for group in groups for language in LANGUAGES for mode in ("evaluate", "control")]
        rng.shuffle(jobs)
        report["orders"].append({"scope": "preloaded", "trial": trial, "jobs": jobs})
        for group, language, mode in jobs:
            result = loaded(group, language, rounds[group, language], mode)
            report["raw_samples"].append({"scope": "preloaded", "phase": "measure", "trial": trial,
                                           "group": group, "language": language, **result})
        print(f"Preloaded randomized sample {trial + 1}/{args.samples} complete.", flush=True)
    for scope in ("process", "preloaded"):
        report["summary"][scope] = {}
        for group in groups:
            report["summary"][scope][group] = {}
            for language in LANGUAGES:
                selected = [s for s in report["raw_samples"] if s["scope"] == scope and s["phase"] == "measure" and s["group"] == group and s["language"] == language and s.get("mode", "evaluate") == "evaluate"]
                timing = "elapsed_ns" if scope == "process" else "ns_per_call"
                summary = {timing: distribution([s[timing] for s in selected]),
                           "peak_rss_bytes": distribution([s["peak_rss_bytes"] for s in selected])}
                if scope == "preloaded":
                    controls = [s["ns_per_call"] for s in report["raw_samples"] if s["scope"] == scope and s["phase"] == "measure" and s["group"] == group and s["language"] == language and s.get("mode") == "control"]
                    summary["control_ns_per_call"] = distribution(controls)
                report["summary"][scope][group][language] = summary
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n")
    print(f"Report: {args.report.resolve()}\nBuilds and exact datasets: {work}", flush=True)


if __name__ == "__main__":
    main()
