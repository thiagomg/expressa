#!/usr/bin/env python3
"""Run warmed, instrumented API probes. This measures allocation requests, not CPU/RSS."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT=Path(__file__).resolve().parent.parent

def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before',type=Path,required=True)
    parser.add_argument('--after',type=Path,required=True)
    parser.add_argument('--manifest',type=Path,required=True)
    parser.add_argument('--repeat',type=int,default=3)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    if args.repeat<1:parser.error('repeat must be positive')
    probes={'before':args.before.resolve(),'after':args.after.resolve()}
    metadata=json.loads(args.manifest.read_text())
    for name,probe in probes.items():
        if digest(probe)!=metadata['libraries'][name]['probe_sha256']:parser.error(f'{name} probe hash differs from manifest')
    metadata.update(scope='instrumented captured API, warmed once per process; includes parse/compile, execution, output validation and cleanup; excludes source file read; requested allocation bytes, not RSS or malloc metadata',repeat=args.repeat,arithmetic='decimal',locale='en',runner_sha256=digest(__file__),clone_graph_note='new feature only: before runtime does not implement clone')
    env=dict(os.environ,EXPRESSA_ARITMETICA='decimal',EXPRESSA_NUMEROS='en')
    records={}
    for scenario,path in [('arithmetic','benchmarks/programs/arithmetic.lep'),('collections','benchmarks/programs/collections.lep'),('arguments','benchmarks/programs/arguments.lep'),('clone_graph','benchmarks/programs/clone/graph.lep'),('sprites','benchmarks/programs/clone/sprites.lep')]:
        path=ROOT/path
        row={'source':str(path.relative_to(ROOT)),'source_sha256':digest(path),'engines':{}}
        for version in (('after',) if scenario=='clone_graph' else ('before','after')):
            for backend in ('ast','bytecode'):
                samples=[]
                for _ in range(args.repeat):
                    result=subprocess.run([str(probes[version]),backend,str(path)],cwd=ROOT,env=env,capture_output=True,text=True,check=True,timeout=60)
                    if result.stderr:raise RuntimeError(result.stderr)
                    samples.append(json.loads(result.stdout))
                row['engines'][version+'_'+backend]=samples
        records[scenario]=row
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps({'metadata':metadata,'workloads':records},indent=2)+'\n')
    print('allocation samples:',sum(len(s) for w in records.values() for s in w['engines'].values()))

if __name__=='__main__':main()
