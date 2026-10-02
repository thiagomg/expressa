'use strict';

const test = require('node:test');
const assert = require('node:assert');
const path = require('path');
const a = require('../src/analise');
const DATA = require('../src/nativas.json');

test('state at the cursor', () => {
  assert.equal(a.stateAtEnd('x = "ab'), 'texto');
  assert.equal(a.stateAtEnd('x = "a\\"b'), 'texto');
  assert.equal(a.stateAtEnd('x = "a" + y'), 'codigo');
  assert.equal(a.stateAtEnd('x // nota'), 'linha');
  assert.equal(a.stateAtEnd('/* a\n b'), 'bloco');
  assert.equal(a.stateAtEnd('a /* b */ c'), 'codigo');
  assert.equal(a.stateAtEnd('#!/usr/bin/env expressa\nx'), 'codigo');
});

test('imports, ignoring comments and strings', () => {
  const src = '// importe "nao"\nimporte "tela"\nm = importe "lib/ajuda"\nx = "importe \\"y\\""\n';
  assert.deepEqual(a.parseImports(src), [
    { alias: null, spec: 'tela', line: 1 },
    { alias: 'm', spec: 'lib/ajuda', line: 2 },
  ]);
  assert.equal(a.importInsertLine(src), 3);
  assert.equal(a.importInsertLine('#!/usr/bin/env expressa\nescreva(1)'), 1);
  assert.equal(a.importInsertLine('escreva(1)'), 0);
});

test('native module or file, like the interpreter', () => {
  assert.equal(a.nativeModule('matriz', DATA.modules), 'matriz');
  assert.equal(a.nativeModule('./matriz', DATA.modules), null);
  assert.equal(a.nativeModule('matriz.lep', DATA.modules), null);
  assert.equal(a.nativeModule('matematica', DATA.modules), null);
  assert.equal(a.resolveImport('lib/ajuda', '/p/main.lep'), path.resolve('/p/lib/ajuda.lep'));
  assert.equal(a.resolveImport('./x.lep', '/p/main.lep'), path.resolve('/p/x.lep'));
});

test('definitions', () => {
  const src = [
    'soma = funcao(a, b) {',
    '    total = a + b',
    '}',
    'x = 1',
    'x == 2',
    'para i de 1 ate 3 {}',
    'm = importe "mat"',
    '// comentado = 2',
    'velocidade_média = função() { 3 }',
    's = "y = 1"',
  ].join('\n');
  const defs = a.parseDefinitions(src, ['se']);
  const by = Object.fromEntries(defs.map((d) => [d.name, d]));
  assert.deepEqual(Object.keys(by).sort(), ['i', 'm', 's', 'soma', 'total', 'velocidade_média', 'x']);
  assert.deepEqual(by.soma.params, ['a', 'b']);
  assert.equal(by.soma.topLevel, true);
  assert.equal(by.total.topLevel, false);
  assert.equal(by.m.kind, 'modulo');
  assert.deepEqual(by['velocidade_média'].params, []);
});

test('completion context', () => {
  const c = a.completionContext;
  assert.deepEqual(c('importe "ma'), { kind: 'importe', prefix: 'ma' });
  assert.deepEqual(c('importe "lib/'), { kind: 'importe', prefix: 'lib/' });
  assert.deepEqual(c('x = "texto'), { kind: 'nenhum' });
  assert.deepEqual(c('x = m::ra'), { kind: 'modulo', alias: 'm', prefix: 'ra' });
  assert.deepEqual(c('lista.ta'), { kind: 'metodo', prefix: 'ta' });
  assert.deepEqual(c('"oi".'), { kind: 'metodo', prefix: '' });
  assert.deepEqual(c('f(x).'), { kind: 'metodo', prefix: '' });
  assert.deepEqual(c('y = 3.'), { kind: 'nome', prefix: '' });
  assert.deepEqual(c('p:'), { kind: 'nenhum' });
  assert.deepEqual(c('pinte(:ve'), { kind: 'nenhum' });
  assert.deepEqual(c('x // esc'), { kind: 'nenhum' });
  assert.deepEqual(c('/* esc'), { kind: 'nenhum' });
  assert.deepEqual(c('esc'), { kind: 'nome', prefix: 'esc' });
  assert.deepEqual(c('a = espa'), { kind: 'nome', prefix: 'espa' });
});

test('call context for signature help', () => {
  const c = a.callContext;
  assert.deepEqual(c('escreva(1, pinte("a", '), { name: 'pinte', alias: null, method: false, active: 1 });
  assert.deepEqual(c('x.pinte(:verde, '), { name: 'pinte', alias: null, method: true, active: 1 });
  assert.deepEqual(c('m::raiz('), { name: 'raiz', alias: 'm', method: false, active: 0 });
  assert.deepEqual(c('f([1, 2], '), { name: 'f', alias: null, method: false, active: 1 });
  assert.deepEqual(c('escreva("a,b", '), { name: 'escreva', alias: null, method: false, active: 1 });
  assert.deepEqual(c('escreva(1)\n'), null);
  assert.deepEqual(c('soma(1,\n    2'), { name: 'soma', alias: null, method: false, active: 1 });
});

test('signature parameters and words', () => {
  assert.deepEqual(a.signatureParams('pinte(texto, frente, fundo) -> texto'), ['texto', 'frente', 'fundo']);
  assert.deepEqual(a.signatureParams('cls()'), []);
  assert.deepEqual(a.wordAt('x = m::raiz(4)', 9), { alias: 'm', name: 'raiz', start: 7, end: 11 });
  assert.deepEqual(a.wordAt('espaço = 1', 3), { alias: null, name: 'espaço', start: 0, end: 6 });
});

test('generated data covers the language', () => {
  const names = DATA.nativas.map((n) => n.name);
  for (const n of ['escreva', 'leia', 'pinte', 'zeros', 'raiz', 'leia_arquivo', 'argumentos']) {
    assert.ok(names.includes(n), n);
  }
  for (const n of DATA.nativas.filter((x) => x.kind === 'funcao')) {
    assert.ok(n.signatures.length > 0, `${n.name} has a signature`);
    assert.ok(n.summary, `${n.name} has a summary`);
  }
  assert.deepEqual(DATA.modules, ['arquivo', 'mat', 'matriz', 'tela']);
  assert.ok(DATA.keywords.controle.includes('se_falhar'));
});
