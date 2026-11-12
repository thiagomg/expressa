#!/usr/bin/env python3
"""User-space perf counters and flat cycle profiles of frozen before/after binaries.

Run only after all builds/tests/other benchmarks finish. Requires permission for
process-local user-space perf events, e.g. perf_event_paranoid=2 on this host.
"""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import random
import statistics
import subprocess
import sys
import tempfile
sys.dont_write_bytecode = True
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from run import ROOT, digest, source_digest

EVENTS=('cycles:u','instructions:u','branches:u','branch-misses:u')


def json_stream(text):
    decoder=json.JSONDecoder()
    rows=[]
    while text.strip():
        row,end=decoder.raw_decode(text.lstrip())
        rows.append(row)
        text=text.lstrip()[end:]
    return rows


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before',type=Path,default=ROOT/'target/bytecode-steps/08-clone/expressa')
    parser.add_argument('--after',type=Path,default=ROOT/'target/bytecode-steps/09-clone-fastpath/expressa')
    parser.add_argument('--repeat',type=int,default=11)
    parser.add_argument('--warmup',type=int,default=2)
    parser.add_argument('--seed',type=int,default=89)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    if args.repeat<1 or args.warmup<0:parser.error('invalid repeat/warmup')
    binaries={'before':args.before.resolve(),'after':args.after.resolve()}
    programs={c:ROOT/f'benchmarks/clone-investigation/programs/{c}.lep' for c in ('arithmetic','closures')}
    profile_programs={c:Path(__file__).with_name('programs')/(c+'.lep') for c in programs}
    files={str(p):digest(p) for p in [*binaries.values(),*programs.values(),*profile_programs.values()]}
    samples={v+'_'+b:{c:[] for c in programs} for v in binaries for b in ('ast','bytecode')}
    rng=random.Random(args.seed)
    for i in range(args.warmup+args.repeat):
        jobs=[(v,b,c) for v in binaries for b in ('ast','bytecode') for c in programs];rng.shuffle(jobs)
        for v,b,c in jobs:
            env=dict(os.environ,EXPRESSA_ARITMETICA='decimal',EXPRESSA_NUMEROS='en',EXPRESSA_BACKEND=b,LC_ALL='C')
            with tempfile.TemporaryDirectory(prefix='expressa-perf-') as temp:
                counter_file=Path(temp)/'counters.json'
                cmd=['perf','stat','-j','-o',str(counter_file),'-e',','.join(EVENTS),'--',str(binaries[v]),str(programs[c])]
                result=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,text=True,timeout=60,check=True)
                if result.stdout!='verdadeiro\n' or result.stderr:raise RuntimeError((cmd,result.stdout,result.stderr))
                raw=counter_file.read_text()
                counters=json_stream(raw)
                if {r['event'] for r in counters}!=set(EVENTS):raise RuntimeError(raw)
                counts={r['event']:float(r['counter-value']) for r in counters}
                if any(r['pcnt-running']<99 for r in counters):raise RuntimeError('Counters multiplexed; do not compare scaled counts silently: '+raw)
                if i>=args.warmup:samples[v+'_'+b][c].append({'counts':counts,'raw':counters})
        print('perf counters round',i+1,flush=True)
    profiles={}
    artifacts=ROOT/'target/clone-fastpath/perf'
    artifacts.mkdir(parents=True,exist_ok=True)
    for v,binary in binaries.items():
        profiles[v]={}
        for c,program in profile_programs.items():
            path=artifacts/(v+'-'+c+'-flat.data')
            cmd=['perf','record','-q','-e','cycles:u','-F','997','--user-callchains','-o',str(path),'--',str(binary),str(program)]
            result=subprocess.run(cmd,cwd=ROOT,env=dict(os.environ,EXPRESSA_ARITMETICA='decimal',EXPRESSA_NUMEROS='en',EXPRESSA_BACKEND='bytecode'),capture_output=True,text=True,timeout=90,check=True)
            if result.stdout!='verdadeiro\n' or result.stderr:raise RuntimeError((cmd,result.stdout,result.stderr))
            report_command=['perf','report','--stdio','--no-children','-g','none','--sort','symbol','--percent-limit','0','-i',str(path)]
            report=subprocess.check_output(report_command,text=True,cwd=ROOT)
            path.with_suffix('.txt').write_text(report)
            profiles[v][c]={'command':cmd,'report_command':report_command,'data_path':str(path),'data_sha256':digest(path),'report':report,'report_sha256':digest(path.with_suffix('.txt'))}
            print('perf profile',v,c,flush=True)
    assert files=={p:digest(Path(p)) for p in files},'inputs changed'
    meta={'captured_at_utc':datetime.now(timezone.utc).isoformat(),'source_sha256':source_digest(),'git_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'perf':subprocess.check_output(['perf','--version'],text=True).strip(),'perf_event_paranoid':Path('/proc/sys/kernel/perf_event_paranoid').read_text().strip(),'runner_sha256':digest(Path(__file__)),'files_sha256':files,'repeat':args.repeat,'warmup':args.warmup,'seed':args.seed,'cpu_affinity':sorted(os.sched_getaffinity(0)),'events':list(EVENTS),'scope':'process-local user-space hardware counts, whole source execution; cycle profiles are flat leaf-symbol samples, bytecode only, separate longer programs; no claim of reliable DWARF call chains or line attribution'}
    medians={v:{c:{e:statistics.median(s['counts'][e] for s in ss) for e in EVENTS} for c,ss in cases.items()} for v,cases in samples.items()}
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps({'metadata':meta,'samples':samples,'medians':medians,'profiles':profiles},indent=2)+'\n')

if __name__=='__main__':
    main()
