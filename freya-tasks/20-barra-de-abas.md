# 20 — Barra de abas com scroll

Prioridade: P2. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

`barra_abas` é uma fila horizontal sem scroll. Muitas abas saem da
janela e não há como chegar ao ×.

Não há Ctrl+Tab / Ctrl+Shift+Tab (`Abas::proxima` está `dead_code`).

## Correção

`ScrollView` horizontal na barra, ou recortar com setas. Ctrl+Tab e
Ctrl+Shift+Tab ciclam (`atalhos.rs`). Clique do meio no rótulo fecha
a aba (se o Freya expuser o botão).

Troca de aba usa a tarefa 06 (não reconstruir o editor).

## Aceite

- 12 abas: dá para chegar à última e fechá-la.
- Ctrl+Tab vai para a aba seguinte e o cursor dela volta (tarefa 06).
