# 22 — Erro de sintaxe sem inundar o status

Prioridade: P2. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

Um `use_future` a cada 400ms faz `parser::parse` do buffer inteiro e
joga a mensagem no status (`main.rs`). Enquanto o aluno escreve `se`
o recado de erro não sai. O GTK sublinha no sítio e não grita a cada
tecla.

## Correção

Debounce ~800ms depois da última edição, ou só mostrar quando o
parse falha **e** o buffer não mudou nesse intervalo. Melhor: marcar
o span no gutter / sublinhado e deixar o status para F5 e recados da
IDE.

Não parsear na thread da UI se o ficheiro for grande; o worker já
está no espírito do proj/06.

## Aceite

- Digitar `escreva("oi` : o status não fica a piscar erro a cada
  letra; depois de parar, ou o sublinhado aparece, ou uma mensagem
  estável.
- Ficheiro válido: status livre para “Salvo …” / “Rodando…”.
- `mensagem_sintaxe` nos testes unitários permanece.
