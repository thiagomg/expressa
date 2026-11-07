# 11 — Gráficos: canvas, teclas, Destacar, F5

Prioridade: P1. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

Cada frame chama `png_folha` / `png_tela` e monta um `ImageViewer`
(`shell.rs`). Animação (`gira.lep`, Conway gráfico) recodifica PNG o
tempo todo.

Só há clique. O GTK (`janela_graf.rs`) manda `tecla`, clique e o X
como `EventoGraf::sair`. Jogos com teclado não andam.

`Graficos::limpar` zera `destacado`. F5 acopla de novo o painel que o
aluno tinha destacado. O GTK foi corrigido para **não** fazer isso.

Destacar é overlay na mesma janela, por cima do editor, não uma
janela irmã.

## Correção

- Raster `rgb32` / `pintar` num canvas Freya (pixels), sem PNG por
  frame. PNG só se algum “salvar imagem” aparecer depois.
- Com o painel focado: teclas → `EventoGraf::tecla`; clique já existe
  (`clique_na_tela`). Coordenadas no espaço da tela (y como o GTK /
  `tela_grafica`).
- `limpar` no F5: apaga folhas/telas/`fechadas`, **mantém**
  `destacado` até Acoplar ou fechar o painel.
- Fechar o × de uma tela: `fechadas` + `EventoGraf::sair`; `mostre`
  dessa id não recria até o próximo F5 (já há teste
  `fecha_ignora_proximo_mostre`).
- Destacar: se o Freya 0.5 tiver janela irmã barata, usar; senão
  overlay que **não** coma o teclado do editor quando o foco está no
  código.

## Aceite

- `exemplos/tela_grafica/gira.lep` atualiza a mesma superfície sem
  engasgo óbvio.
- `clique.lep`: o clique volta em `evento()`.
- Fechar ×: o próximo `mostre` nessa corrida não reabre; F5 reabre.
- Destacar, F5, `mostre`: o painel continua destacado.
