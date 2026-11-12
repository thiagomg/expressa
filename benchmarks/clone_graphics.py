#!/usr/bin/env python3
"""Before/after graphics regressions, raster only; no frame timing or viewer."""
import argparse
from datetime import datetime,timezone
import json
import os
from pathlib import Path
import random
import statistics
import subprocess
import sys
sys.dont_write_bytecode=True
from run import ROOT,digest,source_digest,measure as measure_boolean
from conway import measure as measure_conway,reference,expected_stdout

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before',type=Path,required=True)
    parser.add_argument('--after',type=Path,required=True)
    parser.add_argument('--helper',type=Path,required=True)
    parser.add_argument('--repeat',type=int,default=11)
    parser.add_argument('--warmup',type=int,default=3)
    parser.add_argument('--seed',type=int,default=72)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    if args.repeat<1 or args.warmup<0:parser.error('invalid repeat/warmup')
    binaries={'before':args.before.resolve(),'after':args.after.resolve()}
    helper=args.helper.resolve()
    programs={'sprites':ROOT/'benchmarks/programs/clone/sprites.lep','conway':ROOT/'benchmarks/programs/conway/conway.lep'}
    inputs={str(p):digest(p) for p in [*binaries.values(),helper,*programs.values()]}
    case=('completo',64,25,3)
    expected=expected_stdout(*case,reference(64,28))
    samples={v+'_'+b:{c:[] for c in programs} for v in binaries for b in ('ast','bytecode')}
    rng=random.Random(args.seed)
    for round_number in range(args.warmup+args.repeat):
        jobs=[(v,b,c) for v in binaries for b in ('ast','bytecode') for c in programs];rng.shuffle(jobs)
        for version,backend,c in jobs:
            if c=='sprites':sample=measure_boolean(helper,binaries[version],programs[c],60,backend)
            else:sample=measure_conway(helper,binaries[version],programs[c],backend,case,expected,60)
            if round_number>=args.warmup:samples[version+'_'+backend][c].append(sample)
        print('graphics seed',args.seed,'round',round_number+1,flush=True)
    assert inputs=={p:digest(Path(p)) for p in inputs},'artifacts changed'
    meta={'captured_at_utc':datetime.now(timezone.utc).isoformat(),'git_head':subprocess.check_output(['git','rev-parse','HEAD'],text=True,cwd=ROOT).strip(),'working_source_sha256':source_digest(),'binaries':{v:{'path':str(p),'sha256':digest(p)} for v,p in binaries.items()},'programs':{c:{'path':str(p),'sha256':digest(p)} for c,p in programs.items()},'helper_sha256':digest(helper),'runner_sha256':digest(Path(__file__)),'common_runner_sha256':digest(ROOT/'benchmarks/run.py'),'conway_runner_sha256':digest(ROOT/'benchmarks/conway.py'),'cpu_affinity':sorted(os.sched_getaffinity(0)),'repeat':args.repeat,'warmup':args.warmup,'seed':args.seed,'arithmetic':'decimal','locale':'en','scope':'whole process; raster, no window/IPC; sprites 100 frames x64 stamps; conway completo 64x64, 25 generations +3 generation warmups','expected_stdout':{'sprites':'verdadeiro\n','conway':expected.decode()}}
    medians={e:{c:{k:statistics.median(s[k] for s in ss) for k in ss[0]} for c,ss in cases.items()} for e,cases in samples.items()}
    args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps({'metadata':meta,'samples':samples,'medians':medians},indent=2)+'\n')

if __name__=='__main__':main()
