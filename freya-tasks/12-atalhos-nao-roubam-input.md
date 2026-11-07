# 12 — Atalhos não roubam `leia` / REPL / busca

Prioridade: P1. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

`on_global_key_down` em `main.rs` chama `acao_de_tecla` e
`stop_propagation` para Ctrl+S/W/F, F5, Esc, etc. Com o foco num
`Input` (`leia`, REPL, busca, argumentos, diálogos) o atalho dispara
no IDE.

Esc já fecha busca/ajuda; isso é útil. Ctrl+W no `leia` fecha a aba.
Ctrl+D duplica a linha do editor. F5 com `leia` à espera diz “já está
a correr”.

## Correção

Se o foco for um campo de texto (ou um `Popup`):

- Enter / Esc: o que o campo define (`on_submit`, fechar diálogo).
- Caracteres e Ctrl+A/C/V/X: ficam no campo.
- F5–F11 podem continuar globais (Rodar/Parar/Depurar), se o teste
  manual não atrapalhar o `leia`. Parar (não há F-key hoje no menu
  Parar; o botão Parar) tem de funcionar durante `leia`.

O editor continua a receber Tab e as teclas da tarefa 02.

## Aceite

- `leia("nome:")`, escrever `Ana`, Enter: o programa recebe `Ana`;
  Ctrl+S no campo **não** precisa salvar, mas não pode fechar o
  `leia` sem enviar.
- REPL: Ctrl+W não fecha a aba do código enquanto o Input do REPL
  tem foco (ou fecha só se combinarmos com um atalho explícito
  documentado).
- Ctrl+F abre a busca; Esc fecha a busca e devolve o foco ao editor.
