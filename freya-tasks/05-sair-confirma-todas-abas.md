# 05 — Sair confirma todas as abas sujas

Prioridade: P0. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

`sair` em `comandos.rs` se `alguma_suja` abre Confirmar só para a aba
atual. `fechar_aba(..., Depois::Sair)` chama `close_window` assim que
essa aba some. As outras abas sujas perdem-se.

## Correção

Antes de fechar a janela, percorrer as abas sujas: Salvar / Descartar /
Cancelar por aba (ou um “Salvar todos” + “Descartar todos”). Cancelar
aborta o encerramento. Só `close_window` quando nenhuma aba restar
suja.

Ctrl+Q e Arquivo → Sair usam o mesmo caminho. O X da janela, se o
Freya expuser `on_close_request`, também.

## Aceite

- Duas abas sujas, Ctrl+Q: o diálogo aparece para as duas (em série ou
  com Salvar todos). Cancelar na primeira deixa as duas abertas.
- Uma aba limpa e uma suja: pergunta só da suja, depois fecha.
- Nenhuma suja: fecha na hora; rascunhos da tarefa 04 já estão
  coerentes.
