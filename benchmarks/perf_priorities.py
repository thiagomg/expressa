#!/usr/bin/env python3
"""Repeat flat user-space cycle profiles to prioritize runtime work, without edits."""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import random
import subprocess
import sys
sys.dont_write_bytecode=True
from run import ROOT,digest,source_digest
from conway import reference,expected_stdout


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,default=ROOT/'target/bytecode-steps/09-clone-fastpath/expressa')
    parser.add_argument('--repeat',type=int,default=2)
    parser.add_argument('--seed',type=int,default=90)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    if args.repeat<1:parser.error('repeat must be positive')
    binary=args.binary.resolve()
    programs={c:ROOT/f'benchmarks/clone-fastpath/programs/{c}.lep' for c in ('arithmetic','closures')}
    programs.update({c:ROOT/f'benchmarks/programs/perf-priorities/{c}.lep' for c in ('arguments','sprites','cycles')})
    jobs={c:{'program':p,'args':[],'expected':'verdadeiro\n'} for c,p in programs.items()}
    ref=reference(64,153)
    for mode in ('simulacao','desenho','completo'):
        jobs['conway_'+mode]={'program':ROOT/'benchmarks/programs/conway/conway.lep','args':[mode,'64','150','3','raster'],'expected':expected_stdout(mode,64,150,3,ref).decode()}
    paths=[binary,*{row['program'] for row in jobs.values()}]
    before={str(p):digest(p) for p in paths}
    rng=random.Random(args.seed)
    profiles={c:[] for c in jobs}
    out=ROOT/'target/perf-priorities'
    out.mkdir(parents=True,exist_ok=True)
    env=dict(os.environ,EXPRESSA_BACKEND='bytecode',EXPRESSA_ARITMETICA='decimal',EXPRESSA_NUMEROS='en',LC_ALL='C')
    for iteration in range(args.repeat):
        order=list(jobs);rng.shuffle(order)
        for c in order:
            job=jobs[c];path=out/f'{c}-{iteration}.data'
            cmd=['perf','record','-q','-e','cycles:u','-F','997','--user-callchains','-o',str(path),'--',str(binary),str(job['program']),*job['args']]
            run=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,text=True,timeout=120,check=True)
            if run.stdout!=job['expected'] or run.stderr:raise RuntimeError((cmd,run.stdout,run.stderr))
            report_cmd=['perf','report','--stdio','--no-children','-g','none','--sort','symbol','--percent-limit','0','-i',str(path)]
            report=subprocess.check_output(report_cmd,text=True)
            path.with_suffix('.txt').write_text(report)
            profiles[c].append({'command':cmd,'data_path':str(path),'data_sha256':digest(path),'report_command':report_cmd,'report':report,'report_sha256':digest(path.with_suffix('.txt'))})
            print('profile',iteration+1,c,flush=True)
    assert before=={str(p):digest(p) for p in paths},'inputs changed'
    meta={'captured_at_utc':datetime.now(timezone.utc).isoformat(),'git_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'source_sha256':source_digest(),'binary':{'path':str(binary),'sha256':digest(binary)},'runner_sha256':digest(Path(__file__)),'common_runner_sha256':digest(ROOT/'benchmarks/run.py'),'conway_runner_sha256':digest(ROOT/'benchmarks/conway.py'),'files_sha256':before,'perf':subprocess.check_output(['perf','--version'],text=True).strip(),'perf_event_paranoid':Path('/proc/sys/kernel/perf_event_paranoid').read_text().strip(),'repeat':args.repeat,'seed':args.seed,'cpu_affinity':sorted(os.sched_getaffinity(0)),'backend':'bytecode','arithmetic':'decimal','locale':'en','scope':'flat user-space cycle profiles, whole source processes, no viewer; percentages include inlined opcode work; no claim of reliable call chains or isolated phases','cases':{c:{'program':str(j['program']),'args':j['args'],'expected_stdout':j['expected']} for c,j in jobs.items()}}
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps({'metadata':meta,'profiles':profiles},indent=2)+'\n')

if __name__=='__main__':main()
