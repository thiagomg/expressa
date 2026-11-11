#!/usr/bin/env python3
"""AST/bytecode Conway comparison; whole-process CPU/wall/RSS, raster only.

Uses the existing small native measurement helper. Never measures window FPS.
The drawing case includes its simulation/preparation and retained frame cache.
"""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import random
import signal
import statistics
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
from run import ROOT, digest, source_digest

CASES = [(32, 100, 10), (64, 25, 3), (128, 6, 1)]
MODES = ("simulacao", "desenho", "completo")


def reference(side, steps):
    """Independent B3/S23 reference: generic neighbor enumeration on integers."""
    board = [[int(
        (y % 16 in (2, 3) and x % 16 in (2, 3))
        or (y % 16 == 3 and 9 <= x % 16 <= 11)
        or (y % 16 == 9 and x % 16 == 3)
        or (y % 16 == 10 and x % 16 == 4)
        or (y % 16 == 11 and 2 <= x % 16 <= 4)
    ) for x in range(side)] for y in range(side)]
    for _ in range(steps):
        next_board = [[0] * side for _ in range(side)]
        for y in range(side):
            for x in range(side):
                neighbors = sum(
                    board[(y + dy) % side][(x + dx) % side]
                    for dy in (-1, 0, 1) for dx in (-1, 0, 1) if dx or dy
                )
                next_board[y][x] = int(neighbors == 3 or (board[y][x] and neighbors == 2))
        board = next_board
    checksum = 0
    for row in board:
        for cell in row:
            checksum = (checksum * 31 + cell) % 1000000007
    return sum(map(sum, board)), checksum


def expected_stdout(mode, side, generations, warmup, result):
    phases = "fase aquecimento\n"
    if mode == "desenho":
        phases += "fase preparacao\n"
    alive, checksum = result
    return (phases + "fase execucao\n"
            + f"resultado {mode} {side} {generations} {warmup} [{alive:,}, {checksum:,}]\n").encode()


def measure(helper, binary, program, backend, case, expected, timeout):
    mode, side, generations, warmup = case
    env = dict(os.environ, EXPRESSA_ARITMETICA="decimal", EXPRESSA_NUMEROS="en",
               EXPRESSA_BACKEND=backend)
    with tempfile.TemporaryDirectory(prefix="expressa-conway-") as temporary, \
            tempfile.TemporaryFile() as out, tempfile.TemporaryFile() as err:
        stats = Path(temporary) / "usage.json"
        command = [str(helper), str(stats), str(binary), str(program), mode,
                   str(side), str(generations), str(warmup), "raster"]
        process = subprocess.Popen(command, cwd=ROOT, env=env, stdin=subprocess.DEVNULL,
                                   stdout=out, stderr=err, start_new_session=True)
        try:
            code = process.wait(timeout=timeout)
        except BaseException:
            # The raster path never creates a separate viewer process.
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            raise
        out.seek(0)
        err.seek(0)
        actual, errors = out.read(), err.read()
        if code or actual != expected or errors:
            raise RuntimeError(f"{backend}/{case}: exit={code}, stdout={actual!r}, stderr={errors!r}")
        return json.loads(stats.read_text())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--program", type=Path, required=True)
    parser.add_argument("--helper", type=Path, required=True)
    parser.add_argument("--repeat", type=int, default=15)
    parser.add_argument("--warmup", type=int, default=3, help="process warmups, additional to program generations")
    parser.add_argument("--seed", type=int, default=60)
    parser.add_argument("--timeout", type=float, default=60)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.repeat < 1 or args.warmup < 0 or args.timeout <= 0:
        parser.error("invalid repeat/warmup/timeout")
    binary, program, helper = [p.resolve() for p in (args.binary, args.program, args.helper)]
    for p in (binary, program, helper):
        if not p.is_file():
            parser.error(f"missing file: {p}")
    before_hashes = {str(p): digest(p) for p in (binary, program, helper)}
    reference_results = {side: reference(side, generations + warmup)
                         for side, generations, warmup in CASES}
    cases = [(mode, *config) for config in CASES for mode in MODES]
    key = lambda case: f"{case[0]}-{case[1]}"
    expected = {key(c): expected_stdout(*c, reference_results[c[1]]) for c in cases}
    samples = {backend: {key(c): [] for c in cases} for backend in ("ast", "bytecode")}
    rng = random.Random(args.seed)
    for round_number in range(args.warmup + args.repeat):
        jobs = [(backend, case) for backend in samples for case in cases]
        rng.shuffle(jobs)
        for backend, case in jobs:
            sample = measure(helper, binary, program, backend, case, expected[key(case)], args.timeout)
            if round_number >= args.warmup:
                samples[backend][key(case)].append(sample)
        print(f"seed {args.seed}: round {round_number + 1}/{args.warmup + args.repeat}", flush=True)
    assert before_hashes == {str(p): digest(p) for p in (binary, program, helper)}, "artifacts changed during sampling"
    metadata = {
        "captured_at_utc": datetime.now(timezone.utc).isoformat(),
        "platform": platform.platform(),
        "cpu": next((line.split(':', 1)[1].strip() for line in Path('/proc/cpuinfo').read_text().splitlines()
                     if line.startswith('model name')), platform.processor()) if sys.platform == 'linux' else platform.processor(),
        "cpu_affinity": sorted(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else None,
        "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
        "git_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "working_source_sha256": source_digest(),
        "binary": {"path": str(binary), "sha256": digest(binary), "bytes": binary.stat().st_size},
        "program": {"path": str(program), "sha256": digest(program)},
        "helper": {"path": str(helper), "sha256": digest(helper), "source_sha256": digest(ROOT / 'benchmarks/measure.c')},
        "runner_sha256": digest(Path(__file__)),
        "common_runner_sha256": digest(ROOT / 'benchmarks/run.py'),
        "backends": ["ast", "bytecode"], "arithmetic": "decimal", "locale": "en", "destination": "raster",
        "scope": "whole interpreter process; includes parse/compile, generation warmup, drawing preparation/cache, checksum and cleanup; no window/IPC; no frame percentiles",
        "repeat": args.repeat, "process_warmup": args.warmup, "seed": args.seed,
        "cases": {key(c): {"mode": c[0], "side": c[1], "generations": c[2], "generation_warmup": c[3],
                            "expected_stdout": expected[key(c)].decode()} for c in cases},
    }
    medians = {backend: {case: {metric: statistics.median(s[metric] for s in values)
                               for metric in values[0]} for case, values in cases.items()}
               for backend, cases in samples.items()}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps({"metadata": metadata, "samples": samples, "medians": medians}, indent=2) + "\n")
    for case in medians['ast']:
        a, b = medians['ast'][case], medians['bytecode'][case]
        print(f"{case}: CPU AST={a['cpu_seconds']*1000:.3f}ms VM={b['cpu_seconds']*1000:.3f}ms "
              f"delta={(b['cpu_seconds']/a['cpu_seconds']-1)*100:+.2f}% "
              f"RSS delta={(b['peak_rss_bytes']-a['peak_rss_bytes'])/1024:+.0f}KiB")


if __name__ == "__main__":
    main()
