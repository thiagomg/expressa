#!/usr/bin/env python3
"""Interleave frozen binaries and counterfactual variants on identical inputs."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import random
import statistics
import subprocess
import sys
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from run import ROOT, digest, measure, source_digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifacts', type=Path, default=ROOT/'target/clone-investigation')
    parser.add_argument('--repeat', type=int, default=31)
    parser.add_argument('--warmup', type=int, default=3)
    parser.add_argument('--seed', type=int, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.repeat < 1 or args.warmup < 0:
        parser.error('invalid repeat/warmup')
    build = json.loads((args.artifacts/'manifest.json').read_text())
    binaries = {'before':ROOT/'target/bytecode-steps/08-before-clone/expressa', 'candidate':ROOT/'target/bytecode-steps/08-clone/expressa'}
    binaries.update({name:Path(spec['binary']['path']) for name,spec in build['variants'].items()})
    binaries['historical'] = ROOT/'target/ast-reference-bd654b5/expressa-ast'
    programs = {'arithmetic':ROOT/'benchmarks/programs/arithmetic.lep','closures':ROOT/'benchmarks/programs/closures.lep','sprites':ROOT/'benchmarks/programs/clone/sprites.lep'}
    helper = ROOT/'target/benchmarks/conway-baseline-c2de421/measure'
    hashes = {str(p):digest(p) for p in [*binaries.values(),*programs.values(),helper]}
    backends = {v:('ast',) if v=='historical' else ('ast','bytecode') for v in binaries}
    samples = {v+'_'+b:{c:[] for c in programs} for v in binaries for b in backends[v]}
    rng = random.Random(args.seed)
    for i in range(args.warmup+args.repeat):
        jobs = [(v,b,c) for v in binaries for b in backends[v] for c in programs]
        rng.shuffle(jobs)
        for v,b,c in jobs:
            sample = measure(helper,binaries[v],programs[c],60,b)
            if i >= args.warmup:
                samples[v+'_'+b][c].append(sample)
        print('seed',args.seed,'round',i+1,flush=True)
    assert hashes=={p:digest(Path(p)) for p in hashes},'artifacts changed during timing'
    meta = {'captured_at_utc':datetime.now(timezone.utc).isoformat(),'git_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'source_sha256':source_digest(),'build_manifest':build,'repeat':args.repeat,'warmup':args.warmup,'seed':args.seed,'cpu_affinity':sorted(os.sched_getaffinity(0)),'backends':{v:list(bs) for v,bs in backends.items()},'binary_paths':{v:str(p) for v,p in binaries.items()},'program_paths':{v:str(p) for v,p in programs.items()},'files_sha256':hashes,'runner_sha256':digest(Path(__file__)),'common_runner_sha256':digest(ROOT/'benchmarks/run.py'),'scope':'whole processes, source inputs, default features/release; decimal/en; raster sprites, no viewer; counterfactual variants are not approved implementations'}
    medians = {e:{c:{k:statistics.median(s[k] for s in ss) for k in ss[0]} for c,ss in cases.items()} for e,cases in samples.items()}
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps({'metadata':meta,'samples':samples,'medians':medians},indent=2)+'\n')

if __name__=='__main__':
    main()
