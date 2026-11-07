# 21 — Fonte monoespaçada com fallback

Prioridade: P2. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

`FONT_FAMILY` é `"DejaVu Sans Mono"` (`editor.rs`). No Windows da
Stage 11 essa família muitas vezes não existe; o Skia cai numa fonte
sem `ãéç`. O GTK usa a fonte do SourceView / sistema.

## Correção

Lista de famílias: DejaVu Sans Mono, Cascadia Mono, Consolas,
Liberation Mono, Noto Sans Mono, monospace. A primeira que o Freya
conseguir medir com glifos de `ãéçõ` ganha. Prefs podem guardar o
nome escolhido depois, na mesma chave ou numa chave nova compatível.

O tamanho continua `fonte` em pontos, `fonte_px` como hoje.

## Aceite

- Linux com DejaVu: `função` no editor tem til e cedilha.
- Windows (Stage 11 / tarefa 25): a mesma frase, com Cascadia ou
  Consolas, sem quadrados.
- Console, REPL e painel de variáveis usam a mesma família.
