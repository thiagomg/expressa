# 15 — F1, `::`, completar e assinatura

Prioridade: P1. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

`prefixo_em` / `palavra_em` só pegam caracteres de identificador
(`ajuda.rs`). `matriz::zer` completa `zeros` por acaso (prefixo
`zer`); `m::` lista as primeiras 40 nativas, não os membros de `mat`.

`completar` percorre `builtin_docs` pelo `name`, não
`lang::completion_context` (importe, `mod::`, defs do ficheiro). Só
Ctrl+Espaço; não há lista ao digitar. Não há dica de assinatura.
F12 só procura `nome =` no buffer atual.

F1 em `matriz::zeros` pede ficha de `zeros`.

O GTK: `assist.rs` + `lang.rs`, Ctrl+Espaço e trigger em letra / `.`
/ `::` / `importe "`, F1 fora de um nome abre o catálogo.

## Correção

Reusar a análise de `aula/ui-freya/src/lang.rs` (e a do GTK se já for
pura) para prefixo, `importe "`, `mod::`. Inserir `importe "tela"` só
quando faltar. F1 resolve nome qualificado. Overlay de assinatura com
o cursor dentro da chamada; Esc fecha (já há `Acao::Escapar`).

Lista ao digitar se o `CodeEditor` permitir um popup âncora no caret;
senão Ctrl+Espaço com a lista filtrada pelo prefixo **certo** já
cumpre o aceite do Stage 6.

F12: defs do ficheiro e, se barato, `importe` irmão / `lib/`.

## Aceite

- F1 em `escreva` mostra a mesma ficha do REPL.
- Depois de `importe "matriz"`, completar `matriz::zer` oferece
  `zeros`.
- F1 fora de um nome abre o catálogo; o aluno continua a poder
  escrever no editor (painel ao lado, não modal que coma o teclado).
- Esc fecha ficha, catálogo e sugestões.
