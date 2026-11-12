#!/usr/bin/env python3
"""Compare warmed allocation probes; both stage-08 and stage-09 support clone."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT=Path(__file__).resolve().parents[2]
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--manifest',type=Path,default=ROOT/'target/clone-fastpath/allocations/manifest.json')
    parser.add_argument('--repeat',type=int,default=3)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    if args.repeat<1:parser.error('repeat must be positive')
    m=json.loads(args.manifest.read_text())
    for row in m['libraries'].values():
        assert sha(row['path'])==row['sha256']
        assert sha(row['probe_path'])==row['probe_sha256']
    env=dict(os.environ,EXPRESSA_ARITMETICA='decimal',EXPRESSA_NUMEROS='en')
    programs={c:ROOT/f'benchmarks/programs/{c}.lep' for c in ('arithmetic','collections','arguments','cycles')}
    programs.update({c:ROOT/f'benchmarks/programs/clone/{name}.lep' for c,name in [('clone_graph','graph'),('sprites','sprites')]})
    workloads={}
    for case,p in programs.items():
        row={'source':str(p.relative_to(ROOT)),'source_sha256':sha(p),'engines':{}}
        for v,spec in m['libraries'].items():
            for backend in ('ast','bytecode'):
                ss=[]
                for _ in range(args.repeat):
                    run=subprocess.run([spec['probe_path'],backend,str(p)],cwd=ROOT,env=env,capture_output=True,text=True,timeout=60,check=True)
                    if run.stderr:raise RuntimeError(run.stderr)
                    ss.append(json.loads(run.stdout))
                row['engines'][v+'_'+backend]=ss
        workloads[case]=row
    m.update(repeat=args.repeat,arithmetic='decimal',locale='en',runner_sha256=sha(__file__),scope='same warmed captured-API instrumentation as stage08; allocation requests and live-byte deltas, not RSS; both versions support clone; includes cleanup')
    args.output.write_text(json.dumps({'metadata':m,'workloads':workloads},indent=2)+'\n')
    print('allocation samples:',sum(len(ss) for row in workloads.values() for ss in row['engines'].values()))

if __name__=='__main__':
    main()
