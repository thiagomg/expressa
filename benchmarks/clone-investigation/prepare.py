#!/usr/bin/env python3
"""Build counterfactual clone variants in an isolated checkout, never the runtime tree.

Requires frozen stage-08 source/binary artifacts. Variants are diagnostics, not
proposed implementations: derive reintroduces recursive pair copying and raw
images remove the approved image identity contract.
"""
import argparse
import difflib
import hashlib
import io
import json
from pathlib import Path
import shutil
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[2]
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()


def replace_once(text, before, after):
    assert text.count(before) == 1, before
    return text.replace(before, after, 1)


def variant(sources, mode):
    files = dict(sources)
    p = 'src/runtime/value.rs'
    text = files[p]
    start = text.index('// Pairs own their value')
    end = text.index('/// A list\'s items', start)
    if 'derive' in mode:
        text = text[:start] + text[end:]
        text = replace_once(text, 'pub enum Value {', '#[derive(Clone)]\npub enum Value {')
    elif mode == 'outline':
        start = text.index('                if !matches!(value.as_ref()')
        end = text.index('\n            }\n        }\n    }\n}', start)
        body = text[start:end]
        text = text[:start] + '                clone_pair(self, key, value)' + text[end:]
        pos = text.index('/// A list\'s items')
        helper = '#[inline(never)]\nfn clone_pair(original: &Value, key: &MapKey, value: &Value) -> Value {\n'
        helper += body.replace('Self::', 'Value::').replace('value.as_ref()', 'value').replace('let mut tail = self;', 'let mut tail = original;')
        helper += '\n}\n\n'
        text = text[:pos] + helper + text[pos:]
    if mode.startswith('raw'):
        text = replace_once(text, 'Imagem(Rc<RefCell<Imagem>>)', 'Imagem(Imagem)')
        if 'derive' not in mode:
            text = replace_once(text, 'Self::Imagem(v) => Self::Imagem(Rc::clone(v))', 'Self::Imagem(v) => Self::Imagem(v.clone())')
        text = replace_once(text, 'Value::Imagem(Rc::new(RefCell::new(image)))', 'Value::Imagem(image)')
        text = replace_once(text, '(Value::Imagem(a), Value::Imagem(b)) => *a.borrow() == *b.borrow()', '(Value::Imagem(a), Value::Imagem(b)) => a == b')
        text = replace_once(text, '            let img = img.borrow();\n', '')
        p2 = 'src/runtime/nativas/graf.rs'
        files[p2] = files[p2].replace('img.borrow().clone()', 'img.clone()')
        p2 = 'src/runtime/nativas/imagem.rs'
        # Restore just the old image borrowing implementation; same operation bodies.
        text2 = files[p2].replace(' -> Result<std::cell::Ref<\'a, Imagem>, EvalError>', ' -> Result<&\'a Imagem, EvalError>')
        text2 = text2.replace('Ok(img.borrow())', 'Ok(img)')
        files[p2] = text2
        p2 = 'src/runtime/nativas/clonagem.rs'
        text2 = replace_once(files[p2], '            Value::Imagem(v) => Some(id(4, v)),', '            Value::Imagem(_) => None,')
        text2 = replace_once(text2, '''            Value::Imagem(image) => Value::imagem(
                image
                    .try_borrow()
                    .map_err(|_| "imagem emprestada pelo host")?
                    .clone(),
            ),''', '            Value::Imagem(image) => Value::imagem(image.clone()),')
        files[p2] = text2
    files[p] = text
    return files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--checkout', type=Path, default=Path('/tmp/expressa-clone-investigation'))
    parser.add_argument('--output', type=Path, default=ROOT / 'target/clone-investigation')
    args = parser.parse_args()
    checkout, output = args.checkout.resolve(), args.output.resolve()
    checkout.mkdir(parents=True, exist_ok=True)
    output.mkdir(parents=True, exist_ok=True)
    with tarfile.open(fileobj=io.BytesIO(subprocess.check_output(['git', 'archive', 'HEAD'], cwd=ROOT))) as archive:
        archive.extractall(checkout, filter='data')
    frozen = ROOT / 'target/bytecode-steps/08-clone'
    with tarfile.open(frozen / 'source.tar') as archive:
        archive.extractall(checkout, filter='data')
    names = ['src/runtime/value.rs', 'src/runtime/nativas/graf.rs', 'src/runtime/nativas/imagem.rs', 'src/runtime/nativas/clonagem.rs']
    sources = {name: (checkout / name).read_text() for name in names}
    metadata = {'head_for_workspace': subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(), 'candidate_manifest':json.loads((frozen/'manifest.json').read_text()), 'generator_sha256':sha(__file__), 'variants':{}}
    for mode in ('control', 'derive', 'outline', 'raw-manual', 'raw-derive'):
        print('BUILD', mode, flush=True)
        changed = variant(sources, mode)
        patch = ''
        for name, text in changed.items():
            (checkout/name).write_text(text)
            patch += ''.join(difflib.unified_diff(sources[name].splitlines(True),text.splitlines(True),fromfile='a/'+name,tofile='b/'+name))
        dest = output/mode
        dest.mkdir(exist_ok=True)
        (dest/'variant.patch').write_text(patch)
        command = ['cargo','build','-p','expressa','--release','--lib','--bin','expressa','--offline','--locked','--target-dir',str(output/'build')]
        subprocess.run(command,cwd=checkout,check=True)
        shutil.copy2(output/'build/release/expressa',dest/'expressa')
        shutil.copy2(output/'build/release/libexpressa.rlib',dest/'libexpressa.rlib')
        subprocess.run(['rustc','--edition=2024','-O',str(ROOT/'benchmarks/clone_allocations.rs'),'--extern','expressa='+str(dest/'libexpressa.rlib'),'-L','dependency='+str(output/'build/release/deps'),'-o',str(dest/'allocations')],check=True)
        metadata['variants'][mode] = {k:{'path':str(dest/v),'sha256':sha(dest/v),'bytes':(dest/v).stat().st_size} for k,v in [('binary','expressa'),('library','libexpressa.rlib'),('allocations','allocations'),('patch','variant.patch')]}
        (output/'manifest.json').write_text(json.dumps(metadata,indent=2)+'\n')
    print('Done:',output/'manifest.json',flush=True)

if __name__ == '__main__':
    main()
