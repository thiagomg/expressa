# 24 — `:ident` e keywords iguais ao GTK

Prioridade: P2. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`
e, se preciso, `aula/tree-sitter-expressa` (a spec `expressa.lang` do GTK **não**
se remove).

## Problema

No grammar Freya, `identifier` é `:?[\p{L}_]…`, então `:azul` pinta
como variável. O GTK pinta `:ident` como string (`key-ident`).

`grammar.js` lista as keywords; tem de continuar a coincidir com
`lang.rs` `KEYWORDS` (formas com e sem acento). O teste GTK
`language_spec_lists_every_keyword` é o modelo.

## Correção

Regra `:ident` com highlight de string/símbolo, não `@variable`.
Teste no crate `tree-sitter-expressa` ou em `expressa-aula-freya`
que cada entrada de `KEYWORDS` está no `grammar.js` / highlights.

Não alterar `aula/ui/res/language-specs/expressa.lang` neste item,
salvo se o teste partilhado exigir — o GTK continua a usá-lo.

## Aceite

- `g.cart::plote(função(x) { x }, :azul)`: `:azul` não parece
  identificador de variável.
- `função` / `funcao`, `início` / `inicio` destacam os dois.
- `cargo test -p tree-sitter-expressa` e `-p expressa-aula-freya`
  passam.
