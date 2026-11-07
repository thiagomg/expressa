# 23 — Fila de stdout do F5

Prioridade: P2. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

`sync_channel(256)` em `execucao.rs` bloqueia o VM quando o console
não drena (poll a 16ms). Um `enquanto verdadeiro { escreva("x") }`
enche 256 writes e o Parar só é visto quando o `StopHook` corre —
mas o `send` do stdout pode estar parado no canal cheio, **antes**
do próximo statement, se o `escreva` for o que bloqueia.

`ChannelOut::write` manda cada `write()` do host como um evento.

## Correção

Canal maior, ou unbounded com o teto no **console** (5000 linhas,
tarefa 03). Se o backlog crescer demais, fundir chunks de stdout no
dreno. Parar tem de desbloquear `leia` (já manda linha vazia) e não
ficar atrás de 256 `Saida`.

Teste: laço de `escreva` + Parar em menos de ~1s volta o controle.

## Aceite

- `enquanto verdadeiro { escreva("vivo") }` + Parar: o botão Parar
  encerra; o console ficou nas últimas 5000 linhas.
- Conway continua fluido (já usa `durma`).
