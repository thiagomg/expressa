#!/usr/bin/env python3
"""Measure real CPU time and peak RSS of fresh interpreter processes (Linux/macOS).

One engine records a baseline. Multiple engines are interleaved in randomized
order on identical workloads. No bytecode implementation is provided here.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import signal
import statistics
import subprocess
import tempfile
import time
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parent.parent
EXPECTED = b"verdadeiro\n"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_digest():
    checksum = hashlib.sha256()
    for path in sorted([*ROOT.glob("src/**/*.rs"), *ROOT.glob("docs/nativas/*.lep"), ROOT / "Cargo.toml", ROOT / "Cargo.lock"]):
        checksum.update(str(path.relative_to(ROOT)).encode())
        checksum.update(b"\0")
        checksum.update(path.read_bytes())
    return checksum.hexdigest()


def measure(helper, binary, program, timeout):
    env = os.environ.copy()
    env.update(EXPRESSA_ARITMETICA="decimal", EXPRESSA_NUMEROS="en")
    with tempfile.TemporaryDirectory(prefix="expressa-bench-") as temporary, \
            tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
        stats = Path(temporary) / "usage.json"
        started = time.perf_counter()
        process = subprocess.Popen([str(helper), str(stats), str(binary), str(program)], cwd=ROOT, env=env,
                                   stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr,
                                   start_new_session=True)
        try:
            # The native helper records the interpreter's usage independently.
            # wait4 here is only for waiting on the helper and enforcing a timeout.
            while True:
                pid, status, _ = os.wait4(process.pid, os.WNOHANG)
                if pid:
                    break
                if time.perf_counter() - started >= timeout:
                    os.killpg(process.pid, signal.SIGKILL)
                    os.wait4(process.pid, 0)
                    process.returncode = -9
                    raise RuntimeError(f"{program.name}: exceeded {timeout}s")
                time.sleep(0.001)
            process.returncode = os.waitstatus_to_exitcode(status)
        except BaseException:
            if process.returncode is None:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
            raise
        stdout.seek(0)
        stderr.seek(0)
        actual, errors = stdout.read(), stderr.read()
        if process.returncode or actual != EXPECTED or errors:
            raise RuntimeError(f"{program.name}: exit={process.returncode}, stdout={actual!r}, stderr={errors!r}")
        return json.loads(stats.read_text())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", action="append", required=True, metavar="NAME=BINARY")
    parser.add_argument("--repeat", type=int, default=7)
    parser.add_argument("--warmup", type=int, default=2)
    parser.add_argument("--timeout", type=float, default=30)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if not hasattr(os, "wait4") or platform.system() not in {"Linux", "Darwin"}:
        parser.error("this runner supports Linux/macOS; use equivalent per-process accounting on other hosts")
    if args.repeat < 1 or args.warmup < 0 or args.timeout <= 0:
        parser.error("repeat and timeout must be positive; warmup must be nonnegative")
    engines = {}
    for spec in args.engine:
        name, separator, binary = spec.partition("=")
        if not separator or not name or name in engines:
            parser.error("each engine must have a unique NAME=BINARY")
        path = Path(binary).resolve()
        if not path.is_file() or not os.access(path, os.X_OK):
            parser.error(f"not an executable: {path}")
        engines[name] = path
    helper = ROOT / "target/benchmarks/measure"
    helper.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["cc", "-O2", "-std=c11", "-Wall", "-Wextra", "-Werror",
                    str(ROOT / "benchmarks/measure.c"), "-o", str(helper)], check=True)
    programs = sorted((ROOT / "benchmarks/programs").glob("*.lep"))
    results = {name: {p.stem: [] for p in programs} for name in engines}
    rng = random.Random(args.seed)
    for round_number in range(args.warmup + args.repeat):
        jobs = [(name, p) for name in engines for p in programs]
        rng.shuffle(jobs)
        for name, program in jobs:
            sample = measure(helper, engines[name], program, args.timeout)
            if round_number >= args.warmup:
                results[name][program.stem].append(sample)
    cpu_model = platform.processor()
    cpuinfo = Path("/proc/cpuinfo")
    if cpuinfo.exists():
        cpu_model = next((line.split(":", 1)[1].strip() for line in cpuinfo.read_text().splitlines()
                          if line.startswith("model name")), cpu_model)
    metadata = {
        "captured_at_utc": datetime.now(timezone.utc).isoformat(),
        "platform": platform.platform(), "cpu": cpu_model,
        "cpu_affinity": sorted(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else None,
        "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
        "git_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "working_source_sha256": source_digest(),
        "runner_sha256": digest(Path(__file__)),
        "measurement_helper_source_sha256": digest(ROOT / "benchmarks/measure.c"),
        "measurement_helper_binary_sha256": digest(helper),
        "cc": subprocess.check_output(["cc", "--version"], text=True).splitlines()[0],
        "arithmetic": "decimal", "locale": "en", "mode": "fresh-process end-to-end",
        "repeat": args.repeat, "warmup": args.warmup, "seed": args.seed,
        "binaries": {name: {"path": str(p), "sha256": digest(p), "bytes": p.stat().st_size} for name, p in engines.items()},
        "programs": {p.stem: {"sha256": digest(p), "expected_stdout": EXPECTED.decode()} for p in programs},
    }
    summary = {name: {case: {key: statistics.median(s[key] for s in samples)
                           for key in samples[0]} for case, samples in cases.items()}
               for name, cases in results.items()}
    report = {"metadata": metadata, "samples": results, "medians": summary}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print("engine\tworkload\tCPU ms\twall ms\tpeak RSS KiB")
    for name, cases in summary.items():
        for case, sample in cases.items():
            print(f"{name}\t{case}\t{sample['cpu_seconds'] * 1000:.3f}\t"
                  f"{sample['wall_seconds'] * 1000:.3f}\t{sample['peak_rss_bytes'] / 1024:.0f}")
    if len(engines) == 1:
        print("Baseline only: no AST/bytecode comparison has been made.")
    else:
        baseline = next(iter(engines))
        print(f"Comparison reference: {baseline}")
        for candidate in list(engines)[1:]:
            for case in summary[baseline]:
                before, after = summary[baseline][case], summary[candidate][case]
                speedup = before["cpu_seconds"] / after["cpu_seconds"] if after["cpu_seconds"] else None
                rss_change = after["peak_rss_bytes"] - before["peak_rss_bytes"]
                print(f"{candidate} vs {baseline}: {case}: CPU ratio={speedup}, RSS change={rss_change} bytes")


if __name__ == "__main__":
    main()
