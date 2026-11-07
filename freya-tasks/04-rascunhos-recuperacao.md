# 04 — Rascunhos: fechar o diálogo não apaga

Prioridade: P0. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

`dialogo_recuperar` liga o X / `on_close_request` a
`descartar_rascunhos` (`main.rs`, `shell.rs`). Fechar o popup apaga
`~/.cache/expressa-aula/rascunho-freya`.

`gravar_rascunhos` só escreve abas sujas e **nunca** remove o ficheiro
depois de um save. Um `1.lep` velho reabre Recuperar na próxima
sessão, mesmo com o projeto limpo.

O nome gravado é `{id}.lep`, sem caminho nem título original.

## Correção

- X, Esc e Cancelar: fecham o diálogo e **deixam** os ficheiros.
- Só o botão Descartar apaga a pasta.
- Depois de salvar (ou aba limpa), apagar o rascunho daquela aba.
- Gravar nome original + caminho relativo (ou absoluto) + texto, para
  Recuperar reabrir a aba certa, suja.

## Aceite

- Matar o processo com aba suja → na volta, Recuperar lista o nome do
  ficheiro; Recuperar devolve o texto; Descartar apaga.
- Fechar o X do diálogo na volta: os rascunhos ainda estão no disco;
  a próxima abertura pergunta de novo.
- Ctrl+S numa aba suja: aquele rascunho some; a próxima abertura não
  pergunta se as outras abas estão limpas.
