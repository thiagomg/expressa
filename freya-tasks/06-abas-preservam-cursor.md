# 06 — Abas preservam cursor, scroll e undo

Prioridade: P0. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

`carregar_editor` faz `*editor = new_editor(&aba.texto)`. Trocar de aba
zera cursor, scroll e histórico. `Aba` não guarda posição.

`sincronizar` copia só texto e `dirty`.

## Correção

Uma `CodeEditorData` por aba (mapa `id → editor`) **ou** gravar na
`Aba` a posição UTF-16, a seleção e o identificador de scroll do
`ScrollController`, e restaurar sem `new_editor` se a rope já for a
dessa aba.

`new_editor` fica para ficheiro novo / abrir do disco.

Depende da tarefa 01: o histórico de cada aba só vale se as edições
empilharem de verdade.

## Aceite

- Duas abas, cursor no meio da segunda, voltar à primeira e de novo à
  segunda: o cursor e o scroll estão onde estavam.
- Ctrl+Z na aba A não desfaz a aba B.
- Abrir um `.lep` do disco começa no topo, `dirty` falso.
