# 07 — Layout: editor preenche, REPL no meio

Prioridade: P1. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

A coluna do meio é `CodeEditor` (sem `height(fill)`) + gráficos 240px +
console 160px (`main.rs`). O REPL é um filho de 220px **depois** do
corpo `fill`, não um painel da coluna. O editor encolhe ou o REPL
sobreõe.

Árvore 240px e depurador 280px são larguras fixas, sem alça.

O GTK: árvore | (editor sobre console) | painel; REPL no sítio do
console ou ao lado, com `GtkPaned`.

## Correção

- `CodeEditor::new(...).height(Size::fill())` na coluna do meio.
- Ver → REPL abre um painel **nessa** coluna (acima do console ou no
  lugar dele), 220px, sem cobrir o editor.
- Destacar gráficos continua overlay ou janela (tarefa 11); o dock
  fica entre editor e console, com o editor ainda `fill`.
- Alças simples para árvore e painel (mesmo que não sejam `GtkPaned`).

## Aceite

- Janela maximizada: o editor ocupa a maior parte da altura; console
  ~160px visível.
- Ver → REPL: o editor continua utilizável; o REPL aceita `2+2`.
- F7 / F8 escondem árvore e painel e o editor ganha a largura.
