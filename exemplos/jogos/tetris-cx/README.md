# Tetris em Expressa

Implementação de referência para estudar e comparar estratégias. O código e os comentários estão em português brasileiro. Todos os arquivos do jogo ficam nesta pasta; não há imagens, fontes ou outros recursos para baixar.

## Como jogar

Com a Expressa 0.5 ou mais recente, a partir da raiz do repositório:

```sh
expressa exemplos/tetris-cx
```

Para usar o executável compilado localmente:

```sh
cargo run -p expressa --bin expressa -- exemplos/tetris-cx/tetris.lep
```

Na Aula, abra esta pasta como projeto e execute **principal.lep** com F5. A janela gráfica mede 720×832 pixels, com uma área lógica de 360×416.

| Tecla | Ação |
| --- | --- |
| Esquerda / Direita | Mover; segurar repete o movimento |
| Baixo | Descer uma linha imediatamente; segurar acelera a queda |
| Cima ou X | Girar no sentido horário |
| Z | Girar no sentido anti-horário |
| Espaço | Soltar na posição da peça fantasma e fixar imediatamente |
| P | Pausar ou continuar |
| R | Reiniciar, inclusive depois do fim de jogo |
| Esc | Sair e fechar a janela |

Perder o foco pausa a partida. Pressione P depois de voltar à janela. Fechar a janela também encerra o laço.

## Regras desta versão

- Tabuleiro com 10 colunas e 20 linhas visíveis. A origem das peças nasce uma linha acima do topo; algumas células só aparecem depois de descer.
- Sete formatos: I, O, T, S, Z, J e L. Cada sacola contém os sete tipos, sorteados sem reposição. As três próximas peças aparecem no painel lateral.
- Uma peça apoiada tem 0,45 segundo antes de ser fixada. Mover ou girar não reinicia esse prazo se ela continuar apoiada; sair do apoio reinicia a contagem.
- Limpar uma, duas, três ou quatro linhas vale, respectivamente, 100, 300, 500 ou 800 pontos, multiplicados pelo nível da jogada. A descida manual vale um ponto por linha; a queda direta, dois.
- Cada dez linhas aumentam o nível. A gravidade começa em uma linha a cada 0,8 segundo, reduz 0,06 segundo por nível e tem um mínimo de 0,08 segundo. Segurar Baixo usa 0,04 segundo por linha.
- A partida termina se a nova peça não couber ou se uma peça for fixada com alguma célula acima do topo.

A rotação usa um quadrado de referência e uma pequena tabela de deslocamentos para encontrar espaço junto às paredes e ao piso. É uma regra simplificada, não o sistema oficial SRS. Reserva de peças, combos e pontuação especial por giros ficam como extensões possíveis.

## Organização do código

| Arquivo | Responsabilidade |
| --- | --- |
| [principal.lep](principal.lep) | Início da partida e ligação com o laço gráfico |
| [pecas.lep](pecas.lep) | Catálogo dos formatos e cálculo das rotações |
| [regras.lep](regras.lep) | Tabuleiro, colisão, sorteio, gravidade, limpeza e pontuação |
| [controles.lep](controles.lep) | Teclas, repetição lateral, pausa e reinício |
| [desenho.lep](desenho.lep) | Tabuleiro, peça fantasma, painel e avisos |
| [validar.lep](validar.lep) | Verificações automáticas sem abrir janela |

O estado da partida é um mapa compartilhado. Funções alteram seus campos em vez de reatribuir variáveis externas. O desenho só consulta esse mapa; as regras não dependem de uma janela nem recebem teclas.

### Decisões para comparar com sua implementação

1. **Lista de linhas ou matriz:** aqui usamos `tabuleiro[linha][coluna]`, com 0 para vazio e 1 a 7 para os tipos fixados. Cada linha é criada separadamente, evitando referências compartilhadas por engano.
2. **Rotação calculada ou posições prontas:** cada peça tem quatro células relativas à sua origem. A rotação transforma essas coordenadas; outra abordagem seria guardar antecipadamente as quatro orientações.
3. **Uma regra de colisão:** `cabe` é usada para andar, girar, cair e calcular a peça fantasma. Isso mantém a previsão de queda consistente com a simulação.
4. **Limpeza no próprio tabuleiro ou criação de outro:** a compactação percorre de baixo para cima e só escreve em linhas já examinadas. Depois, cria linhas vazias independentes no topo.
5. **Tempo ou quantidade de quadros:** gravidade, apoio e repetição lateral usam segundos. Os controles limitam o tempo de um quadro a 0,1 segundo para evitar saltos depois de um atraso grande. As regras recebem o tempo como argumento, o que facilita verificá-las.
6. **Entrada contínua ou evento de pressão:** movimentos repetidos usam as teclas mantidas; giros e queda direta usam as recém-pressionadas. Toques completos entre dois quadros continuam sendo reconhecidos.

## Como verificar

Os testes usam uma semente fixa e não precisam abrir uma janela:

```sh
expressa exemplos/tetris-cx/validar.lep
```

Para comparar os dois executores da linguagem:

```sh
expressa --backend ast exemplos/tetris-cx/validar.lep
expressa --backend bytecode exemplos/tetris-cx/validar.lep
```

As verificações cobrem formatos, rotações, independência das cópias, sorteio, colisões, ajustes de giro, limpeza de linhas, níveis, quedas, apoio, fim de jogo, pausa, foco, repetição do teclado e desenho.

Também é possível gerar uma imagem sem abrir janela:

```sh
expressa exemplos/tetris-cx --imagem
```

Ela será gravada em `previa.png` nesta pasta. Para escolher outro destino, passe o caminho depois de `--imagem`.

O programa ativa `define_opcao(:fechar_janelas, verdadeiro)`. Ao terminar, suas janelas são fechadas automaticamente, inclusive em caso de erro ou interrupção. Use uma compilação da Expressa e da Aula que inclua essa opção. O arquivo tetris.lep continua disponível como entrada alternativa.
