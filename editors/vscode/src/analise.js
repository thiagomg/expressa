// Text analysis for Expressa, independent of the VS Code API so it can be
// tested with plain `node --test`.

'use strict';

const path = require('path');

const IDENT = '[\\p{L}_][\\p{L}\\p{N}_]*';
const IDENT_RE = new RegExp(`^${IDENT}$`, 'u');

/** Replace comments and string contents with spaces, keeping offsets and
 * line breaks, so regexes do not match inside them. Quotes are kept. */
function blankCommentsAndStrings(text) {
  let out = '';
  let i = 0;
  const n = text.length;
  const blank = (s) => s.replace(/[^\n]/g, ' ');
  if (text.startsWith('#!')) {
    const end = text.indexOf('\n');
    const stop = end < 0 ? n : end;
    out += blank(text.slice(0, stop));
    i = stop;
  }
  while (i < n) {
    const c = text[i];
    if (c === '/' && text[i + 1] === '/') {
      let j = text.indexOf('\n', i);
      if (j < 0) j = n;
      out += blank(text.slice(i, j));
      i = j;
    } else if (c === '/' && text[i + 1] === '*') {
      let j = text.indexOf('*/', i + 2);
      j = j < 0 ? n : j + 2;
      out += blank(text.slice(i, j));
      i = j;
    } else if (c === '"') {
      let j = i + 1;
      while (j < n && text[j] !== '"' && text[j] !== '\n') j += text[j] === '\\' ? 2 : 1;
      const closed = j < n && text[j] === '"';
      out += '"' + blank(text.slice(i + 1, Math.min(j, n))) + (closed ? '"' : '');
      i = closed ? j + 1 : j;
    } else {
      out += c;
      i += 1;
    }
  }
  return out;
}

/** Lexical state at the end of `text`: 'codigo', 'texto' (inside a
 * string), 'linha' (// comment) or 'bloco' (/* comment). */
function stateAtEnd(text) {
  let state = 'codigo';
  let i = text.startsWith('#!') ? text.indexOf('\n') : 0;
  if (i < 0) return 'linha';
  for (; i < text.length; i++) {
    const c = text[i];
    if (state === 'codigo') {
      if (c === '/' && text[i + 1] === '/') { state = 'linha'; i++; }
      else if (c === '/' && text[i + 1] === '*') { state = 'bloco'; i++; }
      else if (c === '"') state = 'texto';
    } else if (state === 'texto') {
      if (c === '\\') i++;
      else if (c === '"' || c === '\n') state = 'codigo';
    } else if (state === 'linha') {
      if (c === '\n') state = 'codigo';
    } else if (c === '*' && text[i + 1] === '/') { state = 'codigo'; i++; }
  }
  return state;
}

/** `importe "x"` and `alias = importe "x"`, in order. */
function parseImports(text) {
  const out = [];
  const code = blankCommentsAndStrings(text);
  const re = new RegExp(`(?:^|[\\n;])[ \\t]*(?:(${IDENT})[ \\t]*=[ \\t]*)?importe[ \\t]+"`, 'gu');
  for (const m of code.matchAll(re)) {
    // The blanked copy has spaces inside the quotes; read the original.
    const open = m.index + m[0].length;
    const close = text.indexOf('"', open);
    if (close < 0 || text.slice(open, close).includes('\n')) continue;
    const line = text.slice(0, open).split('\n').length - 1;
    out.push({ alias: m[1] || null, spec: text.slice(open, close), line });
  }
  return out;
}

/** Native module name for `spec`, or null when it names a `.lep` file.
 * Mirrors the interpreter: no `/`, `./` or `.lep` means native first. */
function nativeModule(spec, modules) {
  if (spec.includes('/') || spec.endsWith('.lep')) return null;
  return modules.includes(spec) ? spec : null;
}

/** File a non-native `importe` refers to, like resolve_import_path. */
function resolveImport(spec, fromFile) {
  let p = spec;
  if (!path.extname(p)) p += '.lep';
  return path.isAbsolute(p) ? path.normalize(p) : path.resolve(path.dirname(fromFile), p);
}

/** Names defined in a file: functions (`f = funcao(a, b)`), variables and
 * loop variables. First definition wins. */
function parseDefinitions(text, keywords = []) {
  const code = blankCommentsAndStrings(text);
  const reserved = new Set(keywords);
  const defs = new Map();
  const lines = code.split('\n');
  const add = (name, def) => {
    if (reserved.has(name) || defs.has(name)) return;
    defs.set(name, def);
  };
  const fnRe = new RegExp(`^([ \\t]*)(${IDENT})[ \\t]*=[ \\t]*(?:funcao|função)[ \\t]*\\(([^)]*)\\)`, 'u');
  const varRe = new RegExp(`^([ \\t]*)(${IDENT})[ \\t]*=(?!=)`, 'u');
  const loopRe = new RegExp(`(?:^|[^\\p{L}\\p{N}_])para[ \\t]+(${IDENT})[ \\t]+(?:de|em)(?![\\p{L}\\p{N}_])`, 'u');
  lines.forEach((line, i) => {
    let m = line.match(fnRe);
    if (m) {
      const params = m[3].split(',').map((s) => s.trim()).filter(Boolean);
      add(m[2], { name: m[2], kind: 'funcao', params, line: i, topLevel: m[1].length === 0 });
      return;
    }
    m = line.match(varRe);
    if (m && !/^\s*(importe)\b/u.test(line.slice(line.indexOf('=') + 1))) {
      add(m[2], { name: m[2], kind: 'valor', line: i, topLevel: m[1].length === 0 });
    } else if (m) {
      add(m[2], { name: m[2], kind: 'modulo', line: i, topLevel: m[1].length === 0 });
    }
    m = line.match(loopRe);
    if (m) add(m[1], { name: m[1], kind: 'valor', line: i, topLevel: false });
  });
  return [...defs.values()];
}

/** What to complete at the cursor. `before` is the document text up to
 * the cursor. */
function completionContext(before) {
  const state = stateAtEnd(before);
  const line = before.slice(before.lastIndexOf('\n') + 1);
  if (state === 'texto') {
    const m = line.match(/importe[ \t]+"([^"]*)$/u);
    return m ? { kind: 'importe', prefix: m[1] } : { kind: 'nenhum' };
  }
  if (state !== 'codigo') return { kind: 'nenhum' };
  const code = blankCommentsAndStrings(line);
  let m = code.match(new RegExp(`(${IDENT})::(${IDENT})?$`, 'u'));
  if (m) return { kind: 'modulo', alias: m[1], prefix: m[2] || '' };
  // `pessoa:nome` / `:verde`: a key, not a name to complete.
  if (new RegExp(`(?<!:):(${IDENT})?$`, 'u').test(code)) return { kind: 'nenhum' };
  // `x.nome`, `f(x).nome`, `"a".nome` — but not a number like `3.`.
  m = code.match(new RegExp(`([\\p{L}\\p{N}_)\\]"])\\.(${IDENT})?$`, 'u'));
  if (m) {
    const receiver = code.slice(0, m.index + 1);
    if (!/(?:^|[^\p{L}\p{N}_])[0-9][0-9_]*$/u.test(receiver)) {
      return { kind: 'metodo', prefix: m[2] || '' };
    }
  }
  m = code.match(new RegExp(`(${IDENT})$`, 'u'));
  return { kind: 'nome', prefix: m ? m[1] : '' };
}

/** The call around the cursor: `{ name, alias, method, active }` or null.
 * `before` is the document text up to the cursor. */
function callContext(before) {
  const code = blankCommentsAndStrings(before);
  let depth = 0;
  let commas = 0;
  for (let i = code.length - 1; i >= 0; i--) {
    const c = code[i];
    if (c === ')' || c === ']' || c === '}') depth++;
    else if (c === '[' || c === '{') {
      if (depth === 0) return null;
      depth--;
    } else if (c === '(') {
      if (depth > 0) {
        depth--;
        continue;
      }
      const head = code.slice(0, i);
      const m = head.match(new RegExp(`(?:(${IDENT})::)?(${IDENT})[ \\t]*$`, 'u'));
      if (!m) return null;
      const lead = head.slice(0, head.length - m[0].length);
      const method = /\.\s*$/.test(lead) || (m[1] && /\.\s*$/.test(head.slice(0, m.index)));
      return { name: m[2], alias: m[1] || null, method: !!method, active: commas };
    } else if (c === ',' && depth === 0) commas++;
    else if (c === '\n' && code[i - 1] === '\n') return null; // blank line: give up
  }
  return null;
}

/** Parameter names of a documented signature: `pinte(texto, frente) -> texto`. */
function signatureParams(sig) {
  const m = sig.match(/\(([^)]*)\)/);
  if (!m || !m[1].trim()) return [];
  return m[1].split(',').map((s) => s.trim());
}

/** Line where a new `importe` goes: after the last one, else after a shebang. */
function importInsertLine(text) {
  const imports = parseImports(text);
  if (imports.length) return imports[imports.length - 1].line + 1;
  return text.startsWith('#!') ? 1 : 0;
}

/** Identifier (optionally `alias::name`) at `offset` in `text`. */
function wordAt(text, offset) {
  const re = new RegExp(`(?:(${IDENT})::)?(${IDENT})`, 'gu');
  for (const m of text.matchAll(re)) {
    const start = m.index;
    const end = start + m[0].length;
    if (offset >= start && offset <= end) {
      return { alias: m[1] || null, name: m[2], start: start + (m[1] ? m[1].length + 2 : 0), end };
    }
    if (start > offset) break;
  }
  return null;
}

module.exports = {
  IDENT_RE,
  blankCommentsAndStrings,
  stateAtEnd,
  parseImports,
  nativeModule,
  resolveImport,
  parseDefinitions,
  completionContext,
  callContext,
  signatureParams,
  importInsertLine,
  wordAt,
};
