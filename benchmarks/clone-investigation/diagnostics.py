#!/usr/bin/env python3
"""After temporal runs finish: allocation counts, layouts and direct-clone probes.

The direct-clone probe measures a host loop, not the language interpreter. It
cannot establish the cost of Value::clone at particular interpreter call sites.
"""
import argparse
import json
import os
from pathlib import Path
import random
import statistics
import subprocess
import sys
sys.dont_write_bytecode = True
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from run import ROOT,digest,measure


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repeat',type=int,default=15)
    parser.add_argument('--seed',type=int,default=82)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    output=ROOT/'target/clone-investigation'
    build=json.loads((output/'manifest.json').read_text())
    specs={v:{'library':ROOT/f'target/benchmarks/clone-allocations/libexpressa-{old}.rlib','deps':ROOT/'target/release/deps','allocations':ROOT/f'target/benchmarks/clone-allocations/{old}','binary':ROOT/f'target/bytecode-steps/{stage}/expressa'} for v,old,stage in [('before','before','08-before-clone'),('candidate','after','08-clone')]}
    specs.update({v:{'library':Path(row['library']['path']),'deps':output/'build/release/deps','allocations':Path(row['allocations']['path']),'binary':Path(row['binary']['path'])} for v,row in build['variants'].items()})
    layouts={}
    symbols={}
    for v,spec in specs.items():
        dest=output/v
        dest.mkdir(exist_ok=True)
        for name in ('layout','clone_paths'):
            subprocess.run(['rustc','--edition=2024','-O',str(Path(__file__).with_name(name+'.rs')),'--extern','expressa='+str(spec['library']),'-L','dependency='+str(spec['deps']),'-o',str(dest/name)],check=True)
            spec[name]=dest/name
        layouts[v]=json.loads(subprocess.check_output([str(spec['layout'])],text=True))
        symbol_lines=subprocess.check_output(['nm','-S','--size-sort','-C',str(spec['binary'])],text=True).splitlines()
        rows=[line for line in symbol_lines if line.endswith('<expressa::runtime::value::Value as core::clone::Clone>::clone')]
        symbols[v]={'clone_symbols':rows,'clone_symbol_bytes':sum(int(line.split()[1],16) for line in rows),'caution':'out-of-line symbol sizes exclude inlined call sites; diagnostic only'}
    programs={c:ROOT/f'benchmarks/programs/{c}.lep' for c in ('arithmetic','closures')}
    programs['sprites']=ROOT/'benchmarks/programs/clone/sprites.lep'
    allocations={}
    env=dict(os.environ,EXPRESSA_ARITMETICA='decimal',EXPRESSA_NUMEROS='en')
    for v,spec in specs.items():
        allocations[v]={}
        for b in ('ast','bytecode'):
            allocations[v][b]={c:[json.loads(subprocess.check_output([str(spec['allocations']),b,str(p)],text=True,env=env)) for _ in range(3)] for c,p in programs.items()}
    print('Layouts and allocation probes complete; beginning microbenchmark',flush=True)
    files={str(p):digest(p) for spec in specs.values() for k,p in spec.items() if k!='deps'}
    helper=ROOT/'target/benchmarks/conway-baseline-c2de421/measure'
    files[str(helper)]=digest(helper)
    samples={v:{c:[] for c in ('number','list','text')} for v in specs}
    rng=random.Random(args.seed)
    for i in range(3+args.repeat):
        jobs=[(v,c) for v in specs for c in ('number','list','text')];rng.shuffle(jobs)
        for v,c in jobs:
            sample=measure(helper,specs[v]['clone_paths'],Path(c),60,'bytecode')
            if i>=3:samples[v][c].append(sample)
        print('micro round',i+1,flush=True)
    assert files=={p:digest(Path(p)) for p in files}
    medians={v:{c:{k:statistics.median(s[k] for s in ss) for k in ss[0]} for c,ss in cases.items()} for v,cases in samples.items()}
    meta={'arithmetic':'decimal','locale':'en','seed':args.seed,'repeat':args.repeat,'warmup':3,'cpu_affinity':sorted(os.sched_getaffinity(0)),'build_manifest':build,'files_sha256':files,'source_files_sha256':{str(p.relative_to(ROOT)):digest(p) for p in [Path(__file__),Path(__file__).with_name('clone_paths.rs'),Path(__file__).with_name('layout.rs'),ROOT/'benchmarks/clone_allocations.rs',*programs.values()]},'micro_scope':'host direct clone+drop, black-box enum input, 10M number/list or 3M text copies; not interpreter execution','allocation_scope':'same warmed captured-API probe as stage08; requested bytes, not RSS; 3 samples per version/backend/program'}
    args.output.write_text(json.dumps({'metadata':meta,'layouts':layouts,'symbols':symbols,'allocations':allocations,'micro_samples':samples,'micro_medians':medians},indent=2)+'\n')

if __name__=='__main__':
    main()
