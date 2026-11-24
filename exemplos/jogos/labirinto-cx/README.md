# Labirinto em Expressa

Jogo de referência, com código e comentários em português brasileiro. Encontre as três chaves amarelas e chegue à saída, que muda de vermelho para verde quando estiver aberta. Todos os arquivos ficam nesta pasta; os desenhos são feitos por código.

## Executar

Com a Expressa 0.5 ou mais recente, a partir da raiz do repositório:

```sh
expressa exemplos/jogos/labirinto-cx
```

Usando o executável compilado pelo Cargo:

```sh
cargo run -p expressa --bin expressa -- exemplos/jogos/labirinto-cx
```

Na Aula, abra esta pasta como projeto e execute **principal.lep** com F5. A janela padrão mede 736×704 pixels.

| Tecla | Ação |
| --- | --- |
| Setas ou WASD | Andar uma célula; segurar repete o movimento |
| H | Mostrar ou esconder o caminho até a chave mais próxima; depois das três chaves, até a saída |
| P | Pausar ou continuar |
| R | Gerar outro labirinto e reiniciar os contadores |
| Esc | Sair e fechar a janela |

Perder o foco pausa a partida. Ao voltar, pressione P. Fechar a janela também encerra o jogo. Entrar na saída antes de reunir as três chaves não termina a partida.

## Organização

| Arquivo | Responsabilidade |
| --- | --- |
| [principal.lep](principal.lep) | Início e ligação com o laço gráfico |
| [mapa.lep](mapa.lep) | Geração aleatória e busca de caminhos |
| [regras.lep](regras.lep) | Movimento, coleta, saída e estado da partida |
| [controles.lep](controles.lep) | Teclado, repetição, pausa e reinício |
| [desenho.lep](desenho.lep) | Paredes, chaves, jogador, dicas e painel |
| [validar.lep](validar.lep) | Verificações sem abrir janela |

## Estratégias para comparar

1. **Geração com pilha:** uma busca em profundidade visita as salas, que ficam nas coordenadas ímpares. Entre duas salas vizinhas, remove-se uma parede. Quando não há vizinho novo, a pilha permite voltar até a última bifurcação. Todas as passagens ficam conectadas, com um único caminho entre duas posições.
2. **Solução com fila:** a dica usa busca em largura. Ela encontra a chave com menor distância pelas passagens, não pela distância em linha reta. Cada célula guarda a anterior, permitindo reconstruir a rota depois da busca.
3. **Grade e objetos separados:** a grade usa 0 para parede e 1 para passagem. Jogador, chaves e saída ficam fora dela. Assim, coletar uma chave não exige modificar a geometria do labirinto.
4. **Simulação sem janela:** `mova` aceita um deslocamento ortogonal de uma célula e devolve se o movimento foi permitido. Os testes conseguem concluir partidas pelo mesmo código que atende ao teclado.
5. **Tempo independente do desenho:** a repetição começa após 0,18 segundo e segue a cada 0,12 segundo. O tempo de um quadro é limitado a 0,1 segundo para evitar saltos depois de um atraso. Pausa e vitória congelam o relógio.
6. **Dica atualizada por ação:** a busca é refeita ao andar ou ativar a dica, em vez de ser executada em todos os quadros.

O tamanho padrão é 21×15 células. `regras::crie(largura, altura)` permite outros tamanhos ímpares, a partir de 7×7; o desenho calcula suas dimensões com base na grade. Ao usar teclas diagonais, a horizontal tem prioridade. Teclas opostas no mesmo eixo se cancelam.

## Verificar

```sh
expressa exemplos/jogos/labirinto-cx/validar.lep
expressa --backend ast exemplos/jogos/labirinto-cx/validar.lep
expressa --backend bytecode exemplos/jogos/labirinto-cx/validar.lep
```

As verificações incluem diferentes sementes, bordas fechadas, conectividade, ausência de ciclos, coleta das três chaves, vitória, menor caminho, colisões, repetição, pausa, perda de foco, reinício e desenho. Algumas partidas são percorridas automaticamente pela camada de controles.

Para salvar uma prévia sem abrir janela:

```sh
expressa exemplos/jogos/labirinto-cx --imagem
```

O arquivo `previa.png` será criado nesta pasta. Também é possível informar um caminho depois de `--imagem`.

O programa ativa `define_opcao(:fechar_janelas, verdadeiro)`. Ao terminar, suas janelas são fechadas automaticamente, inclusive em caso de erro ou interrupção. Use uma compilação da Expressa e da Aula que inclua essa opção. O arquivo labirinto.lep continua disponível como entrada alternativa.
