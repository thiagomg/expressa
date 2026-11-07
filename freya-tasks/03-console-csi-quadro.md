# 03 — Console com CSI e `quadro`

Prioridade: P0. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

O console Freya só apaga `\x1b[2J` e `\x1b[H` (`console.rs`). O GTK
(`aula/ui/src/console.rs` `write_screen`) trata:

- CUP `H` / `f` (`escreva_em`, overlay a partir da coluna)
- EL `K` (apaga o resto da linha)
- ED `J` / `quadro` (apaga para baixo; `2J` limpa tudo)
- CSI partido entre dois `write()`
- SGR acumulado (`pinte` + `negrito` na sequência seguinte)

`parse_sgr` no Freya recomeça do estilo vazio a cada CSI: `\x1b[32m` +
`\x1b[1m` perde o verde.

Termo, Conway, Forca e `escreva_em` deixam linha velha e lixo ANSI.

## Correção

Portar o modelo `Screen` do GTK (cursor, `start_col`, `partial`,
`held`) para `Console` em texto. Overlay quando `start_col > 0`;
substituir a linha quando é 0. Manter o teto de 5000 linhas.
Acumular SGR até `0` / reset.

Testes equivalentes a `write_screen_like_a_terminal` e
`pinte_nao_aparece_como_lixo`.

## Aceite

- `exemplos/jogos/termo.lep` e `conway_jogo_da_vida.lep`: o quadro
  seguinte apaga o que ficou abaixo / à direita.
- `pinte` / `fundo` / `negrito` pintam trechos, sem códigos na linha.
- Clique numa linha `em arquivo:linha` ainda abre a aba (já existe
  `parse_error_location`).
