#!/usr/bin/env python3
"""Compare native artifact size and build wall time at identical Rivet semantics.

Build Rivet first. This script uses only Python's standard library and the public
CLI; every measured program is also executed and checked before accepting data.
"""
import argparse
import json
import os
from pathlib import Path
import platform
import selectors
import signal
import statistics
import subprocess
import tempfile
import time
import urllib.request


def verify(executable, source, expected):
    if source['entry']['kind'] == 'console':
        result = subprocess.run([str(executable)], capture_output=True, timeout=5, check=True)
        assert result.stdout == expected and not result.stderr
        return
    process = subprocess.Popen([str(executable)], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        with selectors.DefaultSelector() as selector:
            selector.register(process.stdout, selectors.EVENT_READ)
            assert selector.select(5), 'Native server readiness timeout'
        prefix = b'Rivet program server: '
        ready = process.stdout.readline().strip()
        assert ready.startswith(prefix), (ready, process.poll())
        base = ready[len(prefix):].decode()
        for route in source['entry']['routes']:
            with urllib.request.urlopen(base + route['path'], timeout=3) as response:
                assert response.status == route['status']
                assert response.headers.get_content_type() == 'application/json'
                assert json.load(response) == expected[route['path']]
    finally:
        if process.poll() is None:
            process.send_signal(signal.SIGTERM)
        try:
            assert process.wait(timeout=3) == 0
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
            raise
        process.stdout.close()
        process.stderr.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--rivet', default='target/release/rivet')
    parser.add_argument('--samples', type=int, default=3)
    parser.add_argument('--report', default='.rivet/native-benchmark.json')
    args = parser.parse_args()
    if not 1 <= args.samples <= 20:
        parser.error('--samples must be between 1 and 20')
    root = Path(__file__).resolve().parents[1]
    rivet = Path(args.rivet).resolve()
    clang = os.environ.get('RIVET_CLANG', 'clang')
    report = {'objective': 'standalone executable file bytes, exact v1 semantics',
              'baseline': 'same generated LLVM and runtime, -O0, no requested linker stripping',
              'candidate': 'same generated LLVM and runtime, -Oz, linker dead-code stripping',
              'host': {'system': platform.system(), 'machine': platform.machine(),
                       'macos': platform.mac_ver()[0],
                       'clang': subprocess.check_output([clang, '--version'], text=True).splitlines()[0]},
              'samples_per_profile': args.samples,
              'build_timing_scope': 'CLI process start through artifact publication; Rust compiler executable already built',
              'results': []}
    with tempfile.TemporaryDirectory(prefix='rivet-native-size-') as folder:
        folder = Path(folder)
        for example in ['hello', 'server', 'computed']:
            source = json.loads((root / 'examples' / f'{example}.rivet.json').read_text())
            if source['entry']['kind'] == 'http':
                source['entry']['port'] = 0
            file = folder / f'{example}.json'
            file.write_text(json.dumps(source, ensure_ascii=False))
            expected = {'server': {'/hello': {'message': 'hello world'}},
                        'computed': {'/computed': {'details': {'answer': 42, 'ready': True},
                                                  'message': 'The answer is 42'}}}.get(example)
            if example == 'hello':
                expected = subprocess.check_output([str(rivet), 'interpret', str(file)])
                assert expected == b'Hello, world!\n'
            row = {'example': example, 'profiles': {}}
            canonical_ir = None
            for profile in ['baseline', 'size']:
                samples = []
                for index in range(args.samples):
                    output = folder / f'{example}-{profile}-{index}'
                    start = time.perf_counter_ns()
                    subprocess.run([str(rivet), 'build', str(file), '-o', str(output), '--profile', profile],
                                   capture_output=True, check=True, timeout=30)
                    build_ms = (time.perf_counter_ns() - start) / 1_000_000
                    ir = Path(str(output) + '.ll').read_bytes()
                    if canonical_ir is None:
                        canonical_ir = ir
                    assert ir == canonical_ir, 'Profiles must compile identical LLVM input'
                    verify(output, source, expected)
                    samples.append({'bytes': output.stat().st_size, 'build_ms': round(build_ms, 3)})
                row['profiles'][profile] = {'samples': samples,
                    'median_bytes': int(statistics.median(item['bytes'] for item in samples)),
                    'median_build_ms': round(statistics.median(item['build_ms'] for item in samples), 3)}
            baseline = row['profiles']['baseline']['median_bytes']
            candidate = row['profiles']['size']['median_bytes']
            row['bytes_saved'] = baseline - candidate
            row['size_reduction_percent'] = round(100 * (baseline-candidate) / baseline, 2)
            report['results'].append(row)
    destination = Path(args.report)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
