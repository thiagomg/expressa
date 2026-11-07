# 02 — Indentar depois da tecla (não interceptar `m` e `}`)

Prioridade: P0. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

`tratar_tecla` em `edicao.rs` faz `prevent_default` em **todo** `m` e
`}` para recuar `fim`. Isso compete com IME/compose (Stage 0: `função`,
`não`, `início`) e pode duplicar ou perder carácter.

O GTK Aula indenta no `changed` do buffer, depois do carácter estar no
texto. Não prende a tecla no Capture.

Enter hoje também insere `\n` à mão e depois reescreve o arquivo
(`01-editar-com-historico.md`).

## Correção

Deixar o `CodeEditor` inserir a tecla. Depois, se a linha atual for um
fechador (`starts_with_closer`), recuar **só essa linha** com a API de
histórico da tarefa 01. Tab / Shift+Tab continuam recuo de bloco, sem
passar por `set()`.

Não anexar um filtro de tecla que coma identificadores. Dead keys ABNT
passam no editor.

## Aceite

- Digitar `função`, `não`, `início`, `número` sem lutar com o teclado.
- `fim` e `}` sozinhos na linha recuam para o `inicio` / `{`.
- `tempo`, `como`, `enquanto` aceitam `m` no meio da palavra.
- Colar `função não` de outro editor preserva os acentos.
