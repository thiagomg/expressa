# 18 — Pares `inicio`/`fim` e substituir (Ctrl+H)

Prioridade: P2. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

O Stage 3 pedia destaque do par `inicio`/`fim` e `{`/`}` (ignorando
strings/comentários, `blank_comments_and_strings`). Não está no Freya.

Há Ctrl+F; não há substituir (GTK Ctrl+H).

## Correção

Com o cursor num abridor/fechador, pintar o par (sublinhado ou fundo).
Usar o mesmo scan de `indent.rs`. Ctrl+H: barra com termo, substituição,
próxima, substituir, substituir todas no buffer. Esc fecha (tarefa 12).

Editar o buffer com a API da tarefa 01, para o undo sobreviver.

## Aceite

- Cursor em `inicio`: o `fim` correspondente acende; `"início: …"` em
  `poema.lep` não conta.
- Ctrl+H `ola` → `olá`, Substituir todas, Ctrl+Z desfaz de uma vez ou
  passo a passo, mas o texto volta.
