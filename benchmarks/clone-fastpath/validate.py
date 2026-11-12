#!/usr/bin/env python3
"""Check frozen stage09 measurements and their source/binary provenance, no builds."""
import ast
import hashlib
import json
from pathlib import Path
import re
import statistics
import tarfile

ROOT=Path(__file__).resolve().parents[2]
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()


def medians(samples,recorded):
    for key,value in recorded.items():
        assert abs(value-statistics.median(s[key] for s in samples))<1e-6,key


def main():
    m=json.loads(Path(__file__).with_name('manifest.json').read_text())
    for name,h in m['files_sha256'].items():assert sha(ROOT/name)==h,name
    for v,row in m['references'].items():
        directory=ROOT/row['local_artifacts']
        assert sha(directory/'expressa')==row['binary_sha256']
        assert (directory/'expressa').stat().st_size==row['binary_bytes']
        assert sha(directory/'source.tar')==row['archive_sha256']
        with tarfile.open(directory/'source.tar') as archive:
            files={entry.name:archive.extractfile(entry).read() for entry in archive.getmembers() if entry.isfile() and not entry.name.startswith('tests/')}
        h=hashlib.sha256()
        for name in sorted(files,key=Path):h.update(name.encode()+b'\0'+files[name])
        assert h.hexdigest()==row['source_sha256'],v
    assert sha(ROOT/'target/release/expressa')==m['references']['after']['binary_sha256']
    h=hashlib.sha256()
    for p in sorted([*ROOT.glob('src/**/*.rs'),*ROOT.glob('docs/nativas/*.lep'),ROOT/'Cargo.toml',ROOT/'Cargo.lock']):h.update(str(p.relative_to(ROOT)).encode()+b'\0'+p.read_bytes())
    assert h.hexdigest()==m['references']['after']['source_sha256']
    assert sha(ROOT/m['implementation_patch']['path'])==m['implementation_patch']['sha256']
    count=0
    for seed in (85,86,87,88):
        graphics=seed>86
        path=ROOT/f'benchmarks/results/clone-fastpath-{"graphics-" if graphics else ""}{seed}.json'
        j=json.loads(path.read_text());meta=j['metadata']
        assert meta['seed']==seed and meta['warmup']==3 and meta['repeat']==(11 if graphics else 31)
        assert meta['cpu_affinity']==[0]
        assert meta['working_source_sha256']==m['references']['after']['source_sha256']
        assert sha(ROOT/'benchmarks'/('clone_graphics.py' if graphics else 'run.py'))==meta['runner_sha256']
        for spec in meta['binaries'].values():assert sha(spec['path'])==spec['sha256']
        for name,spec in meta['programs'].items():assert sha(spec.get('path',ROOT/f'benchmarks/programs/{name}.lep'))==spec['sha256']
        for engine,cases in j['samples'].items():
            for case,ss in cases.items():
                assert len(ss)==meta['repeat'];count+=len(ss)
                medians(ss,j['medians'][engine][case])
    assert count==2408
    j=json.loads((ROOT/'benchmarks/results/clone-fastpath-perf-89.json').read_text());meta=j['metadata']
    assert sha(Path(__file__).with_name('perf.py'))==meta['runner_sha256']
    assert meta['source_sha256']==m['references']['after']['source_sha256']
    assert meta['perf_event_paranoid']=='2' and meta['cpu_affinity']==[0] and meta['repeat']==11 and meta['warmup']==2
    for name,h in meta['files_sha256'].items():assert sha(name)==h,name
    count=0
    for engine,cases in j['samples'].items():
        for case,ss in cases.items():
            assert len(ss)==11;count+=len(ss)
            medians([s['counts'] for s in ss],j['medians'][engine][case])
            assert all(r['pcnt-running']>=99 for s in ss for r in s['raw'])
    assert count==88
    for cases in j['profiles'].values():
        for row in cases.values():
            assert sha(row['data_path'])==row['data_sha256']
            assert '# Total Lost Samples: 0' in row['report']
            assert sha(Path(row['data_path']).with_suffix('.txt'))==row['report_sha256']
    j=json.loads((ROOT/'benchmarks/results/clone-fastpath-allocations.json').read_text());meta=j['metadata']
    assert sha(Path(__file__).with_name('allocations.py'))==meta['runner_sha256']
    assert sha(ROOT/meta['probe_source'])==meta['probe_source_sha256']
    for spec in meta['libraries'].values():
        assert sha(spec['path'])==spec['sha256']
        assert sha(spec['probe_path'])==spec['probe_sha256']
    count=0
    for case,row in j['workloads'].items():
        assert sha(ROOT/row['source'])==row['source_sha256']
        for b in ('ast','bytecode'):
            a,z=[row['engines'][v+'_'+b] for v in ('before','after')]
            assert len(a)==len(z)==3 and a==z,(case,b)
            assert all(s==a[0] and s['retained_delta_bytes']==0 for s in a)
            count+=len(a)+len(z)
    assert count==72
    for p in Path(__file__).parent.glob('*.py'):ast.parse(p.read_text())
    for p in [Path(__file__).with_name('README.md'),ROOT/'benchmarks/README.md',ROOT/'benchmarks/CLONE.md',ROOT/'benchmarks/BYTECODE-OPTIMIZATIONS.md',ROOT/'lang-tasks/023-copy-and-alias-contract.md']:
        for link in re.findall(r'\[[^\]]+\]\(([^)]+)\)',p.read_text()):
            if '://' not in link and not link.startswith('#'):assert (p.parent/link.split('#')[0]).exists(),(p,link)
    print('Validated source/archive/binary/patch/runner/input/probe hashes; 2,408 CPU/RSS samples, 88 hardware-counter samples, 4 profiles and 72 identical allocation samples; medians, Python syntax and links.')

if __name__=='__main__':
    main()
