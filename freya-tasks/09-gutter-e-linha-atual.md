# 09 — Gutter, clique e linha atual

Prioridade: P1. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

F9 só atualiza a lista “Pontos” no painel. O gutter do
`freya-code-editor` não mostra marcas. Não há destaque da linha
pausada no editor (só o trecho no painel).

O GTK: círculo vermelho no gutter, clique nos ~56px da esquerda, linha
atual amarela (`debug-current`). Marca oca cinzenta se a linha não é
executável.

## Correção

- Clique no gutter (números / margem) alterna o ponto daquela linha
  (mesma regra de caminho da tarefa 08).
- Pintar as linhas com ponto; linha de `Depuracao.linha` enquanto
  `pausado`.
- Se o hook puder dizer “sem instrução”, marca oca + recado no
  console, como o GTK.

Depende de 06 (o editor da aba certa) e 08 (caminhos).

## Aceite

- F9 e clique no gutter na mesma linha: um põe, o outro tira; o
  círculo some.
- F6: a primeira linha executável fica destacada; F10 move o
  destaque.
- Editar o texto **não** apaga os pontos até o aluno os tirar; se as
  linhas andarem, o mínimo aceitável nesta fatia é “ponto pela linha
  numérica atual” (o GTK já tem tracking de insert/delete — pode
  ficar para depois, com nota).
