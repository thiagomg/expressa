#!/usr/bin/env node
// Generates the extension data from the language sources, so the editor
// stays in sync with the interpreter:
//
//   src/lexer/tokens.rs           keyword()   -> keywords
//   src/runtime/nativas/mod.rs    NATIVAS     -> native names, aliases, modules
//   funcoes-nativas.md            ### `name`  -> signatures and docs
//
// Writes src/nativas.json and the keyword/builtin patterns of
// syntaxes/expressa.tmLanguage.json.
//
//   node scripts/gerar.js           write the files
//   node scripts/gerar.js --check   exit 1 if they are out of date

'use strict';

const fs = require('fs');
const path = require('path');

const EXT = path.resolve(__dirname, '..');
const REPO = path.resolve(EXT, '..', '..');
const TOKENS_RS = path.join(REPO, 'src/lexer/tokens.rs');
const NATIVAS_RS = path.join(REPO, 'src/runtime/nativas/mod.rs');
const DOCS_MD = path.join(REPO, 'funcoes-nativas.md');
const OUT_JSON = path.join(EXT, 'src/nativas.json');
const GRAMMAR = path.join(EXT, 'syntaxes/expressa.tmLanguage.json');

function fail(msg) {
  console.error(`gerar: ${msg}`);
  process.exit(1);
}

// `"se" => TokenKind::Se,` and `"senao" | "senão" => TokenKind::Senao,`
function readKeywords() {
  const src = fs.readFileSync(TOKENS_RS, 'utf8');
  const body = src.match(/pub fn keyword\(s: &str\)[\s\S]*?\n\}/);
  if (!body) fail(`keyword() not found in ${TOKENS_RS}`);
  const byKind = {};
  for (const line of body[0].split('\n')) {
    const m = line.match(/^\s*((?:"[^"]+"\s*\|?\s*)+)=>\s*TokenKind::(\w+)/);
    if (!m) continue;
    byKind[m[2]] = [...m[1].matchAll(/"([^"]+)"/g)].map((x) => x[1]);
  }
  if (Object.keys(byKind).length === 0) fail('no keywords parsed from tokens.rs');
  const take = (...kinds) =>
    kinds.flatMap((k) => {
      if (!byKind[k]) fail(`TokenKind::${k} missing from keyword()`);
      const words = byKind[k];
      delete byKind[k];
      return words;
    });
  const groups = {
    constantes: take('Verdadeiro', 'Falso'),
    operadores: take('E', 'Ou', 'Nao', 'Contem'),
    blocos: take('Inicio', 'Fim'),
    funcao: take('Funcao'),
    importe: take('Importe'),
  };
  // Everything else is control flow (se, para, repita, retorne, …).
  groups.controle = Object.values(byKind).flat();
  return groups;
}

// `Nativa { names: &["cls", "limpe_tela"], module: Some("tela") },`
function readNativas() {
  const src = fs.readFileSync(NATIVAS_RS, 'utf8');
  const out = [];
  const re = /Nativa\s*\{\s*names:\s*&\[([^\]]*)\],\s*module:\s*(None|Some\("([^"]+)"\))\s*\}/g;
  for (const m of src.matchAll(re)) {
    const names = [...m[1].matchAll(/"([^"]+)"/g)].map((x) => x[1]);
    out.push({ name: names[0], aliases: names.slice(1), module: m[3] || null });
  }
  if (out.length === 0) fail(`no NATIVAS entries parsed from ${NATIVAS_RS}`);
  return out;
}

// Sections `### \`name\`` up to the next heading, plus the index summaries.
function readDocs() {
  const md = fs.readFileSync(DOCS_MD, 'utf8');
  const summaries = {};
  for (const m of md.matchAll(/^\|\s*\[`([^`]+)`\]\([^)]*\)\s*\|\s*(.+?)\s*\|\s*$/gm)) {
    summaries[m[1]] = m[2];
  }
  const sections = {};
  const parts = md.split(/^(?=#{2,3} )/m);
  for (const part of parts) {
    const h = part.match(/^### `([^`]+)`\s*\n/);
    if (!h) continue;
    let body = part.slice(h[0].length).replace(/\n---\s*$/, '').trim();
    // The module line is shown separately in the editor.
    body = body.replace(/^Módulo `[^`]+`: `importe "[^"]+"`\.\s*\n+/, '');
    const sig = body.match(/```text\n([\s\S]*?)```/);
    const signatures = sig
      ? sig[1].split('\n').map((s) => s.trim()).filter((s) => s && !s.startsWith('//'))
      : [];
    // Not every function is in the index table; fall back to the first
    // sentence of the description.
    const firstText = body
      .replace(/```[\s\S]*?```/g, '')
      .split('\n')
      .map((l) => l.trim())
      .find((l) => l && !l.startsWith('|') && !l.startsWith('**'));
    const fallback = firstText ? firstText.replace(/\.\s.*$/, '.').replace(/\.$/, '') : '';
    sections[h[1]] = {
      signatures,
      summary: summaries[h[1]] || fallback,
      // The signature block is kept apart in `signatures`.
      doc: (sig ? body.replace(sig[0], '') : body).trim().replace(/```text\n/g, '```expressa\n'),
    };
  }
  return sections;
}

function build() {
  const keywords = readKeywords();
  const nativas = readNativas();
  const docs = readDocs();
  const warnings = [];
  const items = nativas.map((n) => {
    const d = docs[n.name];
    if (!d) warnings.push(`sem documentação em funcoes-nativas.md: ${n.name}`);
    else if (d.signatures.length === 0) warnings.push(`sem assinatura (bloco text): ${n.name}`);
    return {
      name: n.name,
      aliases: n.aliases,
      module: n.module,
      kind: 'funcao',
      signatures: d ? d.signatures : [`${n.name}(...)`],
      summary: d ? d.summary : '',
      doc: d ? d.doc : '',
    };
  });
  // Documented names that are not functions (e.g. `argumentos`) are values.
  const known = new Set(nativas.flatMap((n) => [n.name, ...n.aliases]));
  for (const [name, d] of Object.entries(docs)) {
    if (known.has(name)) continue;
    items.push({ name, aliases: [], module: null, kind: 'valor', signatures: [], summary: d.summary, doc: d.doc });
  }
  const modules = [...new Set(nativas.map((n) => n.module).filter(Boolean))].sort();
  return { data: { keywords, modules, nativas: items }, warnings };
}

// Longest first so `se_falhar` wins over `se` in the alternation.
function wordsPattern(words) {
  const alt = [...new Set(words)]
    .sort((a, b) => b.length - a.length || a.localeCompare(b))
    .map((w) => w.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'))
    .join('|');
  return `(?<![\\p{L}\\p{N}_])(?:${alt})(?![\\p{L}\\p{N}_])`;
}

function grammarWith(data) {
  const g = JSON.parse(fs.readFileSync(GRAMMAR, 'utf8'));
  const k = data.keywords;
  const funcs = data.nativas.filter((n) => n.kind === 'funcao').flatMap((n) => [n.name, ...n.aliases]);
  const values = data.nativas.filter((n) => n.kind === 'valor').map((n) => n.name);
  const set = (key, words) => {
    if (!g.repository[key]) fail(`grammar has no repository.${key}`);
    g.repository[key].match = wordsPattern(words);
  };
  set('constantes', k.constantes);
  set('operadores-palavra', k.operadores);
  set('blocos', k.blocos);
  set('palavra-funcao', k.funcao);
  set('palavra-importe', k.importe);
  set('controle', k.controle);
  set('nativas', funcs);
  set('valores-nativos', values);
  return JSON.stringify(g, null, 2) + '\n';
}

function main() {
  const check = process.argv.includes('--check');
  const { data, warnings } = build();
  for (const w of warnings) console.warn(`aviso: ${w}`);
  const outputs = [
    [OUT_JSON, JSON.stringify(data, null, 2) + '\n'],
    [GRAMMAR, grammarWith(data)],
  ];
  let stale = false;
  for (const [file, text] of outputs) {
    const old = fs.existsSync(file) ? fs.readFileSync(file, 'utf8') : '';
    if (old === text) continue;
    if (check) {
      console.error(`desatualizado: ${path.relative(EXT, file)} (rode: node scripts/gerar.js)`);
      stale = true;
    } else {
      fs.mkdirSync(path.dirname(file), { recursive: true });
      fs.writeFileSync(file, text);
      console.log(`gerado: ${path.relative(EXT, file)}`);
    }
  }
  if (stale) process.exit(1);
  const funcs = data.nativas.filter((n) => n.kind === 'funcao').length;
  console.log(`${funcs} funções nativas, ${data.modules.length} módulos, ${Object.values(data.keywords).flat().length} palavras-chave`);
}

main();
