'use strict';

const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { createRequire } = require('node:module');

// Exercise the registered providers with VS Code's document/position contract,
// without requiring a running editor.
class Position {
  constructor(line, character) { Object.assign(this, { line, character }); }
  translate(line, character) { return new Position(this.line + line, this.character + character); }
}
class Range {
  constructor(start, end, endLine, endCharacter) {
    this.start = typeof start === 'number' ? new Position(start, end) : start;
    this.end = typeof start === 'number' ? new Position(endLine, endCharacter) : end;
  }
}
class MarkdownString {
  appendMarkdown() {}
  appendCodeblock() {}
}
class CompletionItem {
  constructor(label, kind) { Object.assign(this, { label, kind }); }
}
class Location {
  constructor(uri, range) { Object.assign(this, { uri, range }); }
}
class Hover {
  constructor(contents, range) { Object.assign(this, { contents, range }); }
}
class SnippetString {
  constructor(value) { this.value = value; }
}
class SignatureInformation {
  constructor(label) { this.label = label; }
}

const providers = {};
const vscode = {
  Position, Range, MarkdownString, CompletionItem, Location, Hover, SnippetString, SignatureInformation,
  SignatureHelp: class { constructor() { this.signatures = []; } }, ParameterInformation: class {},
  CompletionItemKind: {},
  Uri: { file: (fsPath) => ({ scheme: 'file', fsPath }) },
  TextEdit: { insert: (position, newText) => ({ position, newText }) },
  workspace: { textDocuments: [] },
  languages: Object.fromEntries(['CompletionItem', 'Hover', 'SignatureHelp', 'Definition'].map((name) => [
    `register${name}Provider`, (_, provider) => { providers[name] = provider; return { dispose() {} }; },
  ])),
};
const filename = path.resolve(__dirname, '../src/extension.js');
const localRequire = createRequire(filename);
const sandbox = { require: (name) => name === 'vscode' ? vscode : localRequire(name), module: { exports: {} } };
vm.runInNewContext(fs.readFileSync(filename, 'utf8'), sandbox, { filename });
sandbox.module.exports.activate({ subscriptions: [] });

function document(text) {
  const lines = text.split('\n');
  return {
    uri: vscode.Uri.file(path.join(__dirname, 'main.lep')),
    offsetAt: (pos) => lines.slice(0, pos.line).reduce((sum, line) => sum + line.length + 1, 0) + pos.character,
    lineAt: (line) => ({ text: lines[line] }),
    getText(range) {
      return range ? text.slice(this.offsetAt(range.start), this.offsetAt(range.end)) : text;
    },
  };
}
const label = (item) => typeof item.label === 'string' ? item.label : item.label.label;

test('accented imports provide members and signatures without a second import', () => {
  const alias = 'gráfico'.normalize('NFD');
  const header = `${alias} = importe "${'tela_gráfica'.normalize('NFD')}"\n`;
  const doc = document(`${header}${alias}::`);
  const items = providers.CompletionItem.provideCompletionItems(doc, new Position(1, alias.length + 2));
  assert.ok(items.some((item) => label(item) === 'nova'));
  assert.ok(items.every((item) => !item.additionalTextEdits));
  const call = document(`${header}${alias}::nova(`);
  const help = providers.SignatureHelp.provideSignatureHelp(call, new Position(1, alias.length + 7));
  assert.ok(help.signatures.some((sig) => sig.label.startsWith('nova(')));
  const importing = document('importe "tela_grá');
  const imports = providers.CompletionItem.provideCompletionItems(importing, new Position(0, 17));
  assert.ok(imports.some((item) => label(item) === 'tela_gráfica'));
});

test('error capture completion and navigation stay inside the handler', () => {
  const doc = document('valor = numero("x") se_falhar falha {\n  escreva(falha)\n}\nescreva(falha)');
  const inside = new Position(1, 13);
  const outside = new Position(3, 12);
  assert.ok(providers.CompletionItem.provideCompletionItems(doc, inside).some((item) => label(item) === 'falha'));
  assert.ok(!providers.CompletionItem.provideCompletionItems(doc, outside).some((item) => label(item) === 'falha'));
  assert.equal(providers.Definition.provideDefinition(doc, inside).range.line, 0);
  assert.equal(providers.Definition.provideDefinition(doc, outside), null);
});

test('Unicode normalization preserves hover ranges and definition locations', () => {
  const name = 'ação'.normalize('NFD');
  const doc = document(`ação = função(x) { x }\n${name}(1)`);
  const pos = new Position(1, name.length - 1);
  const hover = providers.Hover.provideHover(doc, pos);
  assert.equal(hover.range.start.character, 0);
  assert.equal(hover.range.end.character, name.length);
  assert.equal(providers.Definition.provideDefinition(doc, pos).range.line, 0);
  const help = providers.SignatureHelp.provideSignatureHelp(doc, new Position(1, name.length + 1));
  assert.equal(help.signatures[0].label, 'ação(x)');
  const completing = document(`ação = função(x) { x }\n${name}`);
  const item = providers.CompletionItem.provideCompletionItems(completing, new Position(1, name.length))
    .find((candidate) => label(candidate) === 'ação');
  assert.equal(item.filterText, name);
  assert.equal(item.range.start.character, 0);
  assert.equal(item.range.end.character, name.length);
});
