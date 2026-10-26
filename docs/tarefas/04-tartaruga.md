# Tartaruga em Expressa

A tartaruga Logo é um exemplo, escrito em Expressa, não um módulo nativo. A biblioteca fica em `exemplos/lib/tartaruga.lep`. Os programas `exemplos/jogos/tartaruga_quadrado.lep` e `tartaruga_estrela.lep` importam com `importe "../lib/tartaruga"`.

Ângulo 0 olha para cima. `vire` gira para a direita (horário). A casa é o centro da tela. Por baixo usa `tela_grafica` (pixels) e `mat` (seno/cosseno). A tartaruga é um mapa (`:x`, `:y`, `:angulo`, `:baixo`, `:cor`, `:tela`).

Chamadas: `nova`, `nova_tamanho`, `ande`, `vire`, `vire_esquerda`, `subir`, `baixar`, `cor`, `olhe`, `casa`, `apague`, `mostre`, `salve`, `velocidade`.

`velocidade(t, segundos)` usa o `durma` que já existe. Cada pedaço de 8 pixels chama `mostre` e espera. `0` desenha a reta inteira de uma vez. Os jogos de exemplo usam `0.02`. Com velocidade maior que zero, `exemplos/lib/tartaruga.png` (32×32) é carimbado no lugar da tartaruga e gira com `vire`. A figura sai da lista de traços antes do próximo passo, então não mancha o desenho. `salve` também não grava a figura.
