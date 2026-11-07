# 01 — Editar com histórico (sem `set()`)

Prioridade: P0. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

`edicao.rs` aplica recuo, comentário, duplicar, mover e reindentar o
arquivo via `aplicar_texto` → `CodeEditorData::set()`. No
`freya-code-editor` 0.5, `set` esvazia a rope e insere o texto novo
**sem** empilhar no histórico. `is_edited()` compara o índice do
histórico com o último save.

Consequências:

- Ctrl+Z depois de Enter/Tab/`fim`/Ctrl+/ desfaz o estado errado ou
  corrompe o buffer (o histórico guarda um `insert("\n")` sobre um
  texto que `set` já substituiu).
- Ctrl+/ num arquivo salvo não marca a aba suja; fechar descarta o
  comentário.
- Tab, duplicar, Alt+↑/↓ e Ctrl+Alt+\\ têm o mesmo furo.

Arquivos: `aula/ui-freya/src/edicao.rs`, `aula/ui-freya/src/editor.rs`
(`aplicar_texto`).

## Correção

Aplicar o trecho alterado com `insert` / `remove` na faixa de UTF-16
(ou um replace que empilhe `HistoryChange`). Chamar `parse()` depois.
Não usar `set()` para edição do aluno. Recolocar o cursor sem
reconstruir o `CodeEditorData`.

`aplicar_texto` pode ficar para carregar arquivo / recuperar rascunho,
com `mark_as_saved` quando o conteúdo veio do disco.

## Aceite

- Digitar `se a {` Enter `x` Ctrl+Z volta ao Enter, o segundo Ctrl+Z
  tira o `{` (ou o último carácter), sem sumir o arquivo.
- Ctrl+/ num `.lep` salvo marca `●` na aba; Descartar no fechar mostra
  o diálogo; Salvar grava os `//`.
- `cargo test -p expressa-aula-freya --offline` segue verde; acrescentar
  um teste de `indentar_linha` / comentário que não dependa de `set`.
