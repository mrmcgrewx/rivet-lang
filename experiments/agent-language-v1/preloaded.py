"""Auditable benchmark-only Rivet I/O substitutions; solver LLVM is unchanged."""
import json
from pathlib import Path
import shutil

from grade import C_FLAGS, HARNESS, run
from protocol import PROJECT, TASKS, digest

BOUNDARIES = {
    "main": "rv_agent_solve", "rv2_read_array": "bench_read_array",
    "rv2_read_list": "bench_read_list", "rv2_read_int": "bench_read_int",
    "rv2_return": "bench_return_array", "rv2_return_list": "bench_return_list",
    "rv2_return_float": "bench_return_float", "rv_cleanup": "bench_cleanup",
}


def build_preloaded(task, standalone, directory):
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    original_path = Path(str(standalone) + ".ll")
    original = original_path.read_text()
    transformed = original
    for old, new in BOUNDARIES.items():
        transformed = transformed.replace(f"@{old}(", f"@{new}(")
    restored = transformed
    for old, new in BOUNDARIES.items():
        restored = restored.replace(f"@{new}(", f"@{old}(")
    ticks = original.count("call void @rv2_tick()")
    assert restored == original and transformed.count("call void @rv2_tick()") == ticks > 0
    llvm = directory / "adapted.ll"
    llvm.write_text(transformed)
    for name in ("runtime.c", "runtime.h"):
        shutil.copyfile(PROJECT / "native" / name, directory / name)
    obj = directory / "solver.o"
    executable = directory / "preloaded"
    commands = [["clang", "-O3", "-Wno-override-module", "-c", llvm, "-o", obj],
                ["clang", *C_FLAGS, f"-DTASK={TASKS[task]}", "-I", HARNESS, "-I", directory,
                 HARNESS / "driver.c", HARNESS / "rivet_backend.c", obj, "-o", executable]]
    logs = []
    for command in commands:
        result = run(command)
        logs.append(result)
        if result["exit_code"] != 0:
            raise RuntimeError(json.dumps(result))
    return {"executable": str(executable), "logs": logs, "generated_tick_call_sites": ticks,
            "original_llvm_sha256": digest(original_path), "adapted_llvm_sha256": digest(llvm),
            "reversible_boundary_only_substitution": True, "boundaries": BOUNDARIES,
            "runtime_sha256": digest(directory / "runtime.c")}
