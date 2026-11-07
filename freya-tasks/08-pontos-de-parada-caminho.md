# 08 — Pontos de parada usam o caminho do VM

Prioridade: P1. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

No `iniciar`, os pontos passam por `abs_no_workspace` (caminho
absoluto da cópia `exec-freya`). F9 com o programa a correr manda
`ponto {rel}:{linha}` (`comandos.rs`). O `ChannelDebugger` guarda
`(arquivo, linha)` como o VM vê o ficheiro (absoluto). O ponto novo
não dispara nesta execução.

O GTK documenta: F9 durante a corrida vale na **próxima** execução.

## Correção

Escolher uma das duas, e só uma:

1. Igual ao GTK: F9 atualiza a lista da UI e o status diz que vale na
   próxima execução; não mandar `ponto`/`remover` no stream ao vivo.
2. Mandar o **mesmo** caminho absoluto que `iniciar` inseriu
   (`ws.join(rel)`).

A lista em `Depuracao.pontos` guarda o relativo do projeto (estável
entre corridas). A tradução para absoluto acontece só na hora de
armar o hook.

## Aceite

- F9 na linha 2, F6, Continuar: para na linha 2.
- F6 já a correr, F9 noutra linha: ou o status avisa “próxima
  execução” e esta corrida ignora, ou a corrida atual passa a
  respeitar o ponto — o teste `depura_para_no_ponto` continua a
  passar.
- F9 de novo na mesma linha tira o ponto da lista.
