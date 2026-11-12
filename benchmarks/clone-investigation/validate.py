#!/usr/bin/env python3
"""Validate recorded inputs, raw medians and allocation/layout conclusions.

Requires the local frozen binaries and probes; does not run benchmarks or builds.
"""
import ast
import hashlib
import json
from pathlib import Path
import re
import statistics
import tarfile

ROOT=Path(__file__).resolve().parents[2]
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()


def main():
    manifest=json.loads(Path(__file__).with_name('manifest.json').read_text())
    for name,expected in manifest['files_sha256'].items():
        assert sha(ROOT/name)==expected,name
    digest=hashlib.sha256()
    with tarfile.open(ROOT/'target/bytecode-steps/08-clone/source.tar') as archive:
        files={m.name:archive.extractfile(m).read() for m in archive.getmembers() if m.isfile() and not m.name.startswith('tests/')}
    for name in sorted(files,key=Path):digest.update(name.encode()+b'\0'+files[name])
    assert digest.hexdigest()==manifest['references']['candidate']['source_sha256']
    assert sha(ROOT/'target/bytecode-steps/08-clone/expressa')==manifest['references']['candidate']['binary_sha256']
    assert sha(Path(__file__).with_name('prepare.py'))==manifest['build']['generator_sha256']
    for version,row in manifest['build']['variants'].items():
        for artifact in row.values():
            assert sha(artifact['path'])==artifact['sha256'],(version,artifact)
    total=0
    for seed in (80,81,83,84):
        long=seed>81
        j=json.loads((ROOT/f'benchmarks/results/clone-investigation-{"long-" if long else ""}{seed}.json').read_text())
        m=j['metadata']
        assert m['seed']==seed and m['cpu_affinity']==[0]
        assert m['repeat']==(11 if long else 31) and m['warmup']==(2 if long else 3)
        assert sha(Path(__file__).with_name('measure_long.py' if long else 'measure.py'))==m['runner_sha256']
        assert sha(ROOT/'benchmarks/run.py')==m['common_runner_sha256']
        for p,h in m['files_sha256'].items():assert sha(p)==h,p
        for engine,cases in j['samples'].items():
            for case,ss in cases.items():
                assert len(ss)==m['repeat'];total+=len(ss)
                for key,value in j['medians'][engine][case].items():
                    assert abs(value-statistics.median(s[key] for s in ss))<1e-6,(seed,engine,case,key)
    assert total==3142,total
    j=json.loads((ROOT/'benchmarks/results/clone-investigation-diagnostics.json').read_text());m=j['metadata']
    for key in ('files_sha256','source_files_sha256'):
        for p,h in m[key].items():assert sha(ROOT/p)==h,p
    assert m['arithmetic']=='decimal' and m['locale']=='en' and m['cpu_affinity']==[0]
    count=0
    for v,cases in j['micro_samples'].items():
        for case,ss in cases.items():
            assert len(ss)==15;count+=len(ss)
            for key,value in j['micro_medians'][v][case].items():assert abs(value-statistics.median(s[key] for s in ss))<1e-6
    assert count==315
    count=0
    for backends in j['allocations'].values():
        for cases in backends.values():
            for ss in cases.values():
                assert len(ss)==3;count+=len(ss)
                assert all(s==ss[0] and s['retained_delta_bytes']==0 for s in ss)
    assert count==126
    for b in ('ast','bytecode'):
        before,candidate,raw=[j['allocations'][v][b]['sprites'][0] for v in ('before','candidate','raw-manual')]
        assert candidate['allocations']-raw['allocations']==6502
        assert candidate['requested_bytes']-raw['requested_bytes']==6502*48
        assert candidate['allocations']-before['allocations']==6506
        assert raw['requested_bytes']-before['requested_bytes']==53808
    assert all(row['value_bytes']==32 for row in j['layouts'].values())
    assert j['layouts']['candidate']['map_storage_bytes']-j['layouts']['before']['map_storage_bytes']==8
    for p in Path(__file__).parent.glob('*.py'):ast.parse(p.read_text())
    for p in [Path(__file__).with_name('README.md'),ROOT/'benchmarks/CLONE.md',ROOT/'benchmarks/README.md']:
        for link in re.findall(r'\[[^\]]+\]\(([^)]+)\)',p.read_text()):
            if '://' not in link and not link.startswith('#'):assert (p.parent/link.split('#')[0]).exists(),(p,link)
    print('Validated hashes, frozen reference source/binary, 3,457 temporal samples/medians, 126 allocation samples, layout/deltas, Python syntax and report links.')

if __name__=='__main__':
    main()
