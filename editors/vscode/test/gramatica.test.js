// Tokenizes Expressa with the same TextMate engine VS Code uses, so a bad
// regex in the grammar fails here instead of silently in the editor.

'use strict';

const test = require('node:test');
const assert = require('node:assert');
const fs = require('fs');
const path = require('path');
const oniguruma = require('vscode-oniguruma');
const textmate = require('vscode-textmate');

const EXT = path.resolve(__dirname, '..');
const REPO = path.resolve(EXT, '..', '..');

async function loadGrammar() {
  const wasm = fs.readFileSync(require.resolve('vscode-oniguruma/release/onig.wasm'));
  await oniguruma.loadWASM(wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength));
  const registry = new textmate.Registry({
    onigLib: Promise.resolve({
      createOnigScanner: (p) => new oniguruma.OnigScanner(p),
      createOnigString: (s) => new oniguruma.OnigString(s),
    }),
    loadGrammar: async () =>
      textmate.parseRawGrammar(
        fs.readFileSync(path.join(EXT, 'syntaxes/expressa.tmLanguage.json'), 'utf8'),
        'expressa.tmLanguage.json',
      ),
  });
  return registry.loadGrammar('source.expressa');
}

/** [{ text, scope }] with the innermost scope of each token. */
function tokenize(grammar, src) {
  const out = [];
  let state = textmate.INITIAL;
  for (const line of src.split('\n')) {
    const r = grammar.tokenizeLine(line, state);
    for (const t of r.tokens) {
      out.push({ text: line.slice(t.startIndex, t.endIndex), scope: t.scopes[t.scopes.length - 1], all: t.scopes.join(' ') });
    }
    state = r.ruleStack;
  }
  return out;
}

function scopeOf(tokens, text) {
  const t = tokens.find((x) => x.text.trim() === text);
  assert.ok(t, `token ${JSON.stringify(text)} not found in ${JSON.stringify(tokens.map((x) => x.text))}`);
  return t.scope;
}

test('grammar scopes', async () => {
  const g = await loadGrammar();
  const src = [
    '#!/usr/bin/env expressa',
    'importe "tela"',
    'm = importe "mat"',
    '/* bloco',
    '   continua */',
    'soma = função(a, b) {',
    '    a + b // fim de linha',
    '}',
    'senão_x = 1_000.5',
    'se_falhar_não = verdadeiro',
    'para ação de 1 até 3 { escreva(pinte("oi\\n", :verde)) }',
    'x = modulo::raiz(4) se_falhar 0',
    'p = pessoa:nome',
    'senão',
    'escreva(argumentos[1], formate("{:<8}", "a"))',
    'r = minha_funcao(2)',
  ].join('\n');
  const t = tokenize(g, src);
  assert.match(scopeOf(t, '#!/usr/bin/env expressa'), /^comment\.line\.shebang/);
  assert.equal(scopeOf(t, 'importe'), 'keyword.control.import.expressa');
  assert.match(scopeOf(t, 'tela'), /^string\.quoted/);
  assert.match(t.find((x) => x.text.includes('continua')).all, /comment\.block/);
  assert.equal(scopeOf(t, 'soma'), 'entity.name.function.expressa');
  assert.equal(scopeOf(t, 'função'), 'storage.type.function.expressa');
  assert.match(t.find((x) => x.text.includes('fim de linha')).all, /comment\.line\.double-slash/);
  // Accented identifiers that start with a keyword are plain names.
  assert.ok(!/keyword/.test(scopeOf(t, 'senão_x')), 'senão_x is not a keyword');
  assert.ok(!/keyword/.test(scopeOf(t, 'se_falhar_não')), 'se_falhar_não is not a keyword');
  assert.equal(scopeOf(t, '1_000.5'), 'constant.numeric.decimal.expressa');
  assert.equal(scopeOf(t, 'verdadeiro'), 'constant.language.boolean.expressa');
  assert.equal(scopeOf(t, 'ação'), 'source.expressa');
  assert.equal(scopeOf(t, 'até'), 'keyword.control.expressa');
  assert.equal(scopeOf(t, 'pinte'), 'support.function.builtin.expressa');
  assert.equal(scopeOf(t, '\\n'), 'constant.character.escape.expressa');
  assert.equal(scopeOf(t, ':verde'), 'constant.other.symbol.expressa');
  assert.equal(scopeOf(t, 'modulo'), 'entity.name.namespace.expressa');
  assert.equal(scopeOf(t, 'se_falhar'), 'keyword.control.expressa');
  assert.equal(scopeOf(t, ':nome'), 'constant.other.symbol.expressa');
  assert.equal(scopeOf(t, 'senão'), 'keyword.control.expressa');
  assert.equal(scopeOf(t, 'argumentos'), 'variable.language.expressa');
  assert.equal(scopeOf(t, '{:<8}'), 'constant.other.placeholder.expressa');
  assert.equal(scopeOf(t, 'minha_funcao'), 'entity.name.function.call.expressa');
});

test('every example in the repository tokenizes', async () => {
  const g = await loadGrammar();
  const files = [];
  const walk = (dir) => {
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, e.name);
      if (e.isDirectory()) walk(full);
      else if (e.name.endsWith('.lep')) files.push(full);
    }
  };
  walk(path.join(REPO, 'exemplos'));
  assert.ok(files.length > 10, `few examples found: ${files.length}`);
  for (const f of files) {
    const tokens = tokenize(g, fs.readFileSync(f, 'utf8'));
    assert.ok(tokens.length > 0, f);
    // An unterminated string or comment would swallow the rest of the file.
    const last = tokens[tokens.length - 1];
    assert.ok(!/string|comment\.block/.test(last.scope) || last.text.endsWith('"') || last.text.endsWith('*/'),
      `${path.relative(REPO, f)} ends inside ${last.scope}`);
  }
});
