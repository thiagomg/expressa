# Cobrinha em Expressa

Implementação de Snake para estudar e comparar estratégias. O código, os comentários e a interface estão em português brasileiro. Coma as frutas, cresça e evite as paredes e o próprio corpo. Todos os arquivos ficam nesta pasta, sem imagens ou outros recursos externos.

## Executar

Com a Expressa 0.5 ou mais recente, a partir da raiz do repositório:

```sh
expressa exemplos/jogos/snake-cx
```

Usando o executável compilado pelo Cargo:

```sh
cargo run -p expressa --bin expressa -- exemplos/jogos/snake-cx
```

Na Aula, abra esta pasta como projeto e execute **principal.lep** com F5. A janela padrão mede 640×640 pixels. O jogo espera uma direção válida para começar; a cobrinha começa apontando para a direita.

| Tecla | Ação |
| --- | --- |
| Setas ou WASD | Escolher a próxima direção |
| P | Pausar ou continuar |
| R | Reiniciar e voltar à espera inicial |
| Esc | Sair e fechar a janela |

Não é preciso segurar uma tecla para continuar andando. Virar diretamente para o sentido oposto é recusado. Perder o foco pausa a partida e descarta curvas pendentes; pressione P para continuar. Fechar a janela também encerra o jogo.

## Regras desta versão

- Tabuleiro de 24×18 células, sem passagem de uma borda para a outra.
- Corpo inicial com três células. Cada fruta aumenta o corpo em uma célula e vale dez pontos.
- A cada cinco frutas, o nível aumenta. O intervalo entre passos começa em 0,16 segundo, diminui 0,015 segundo por nível e nunca fica abaixo de 0,07 segundo.
- Bater na parede ou no corpo encerra a partida. Entrar na célula da cauda é permitido quando a cauda sai no mesmo passo, sem crescimento.
- Ocupar todo o tabuleiro encerra a partida com vitória, sem tentar sortear outra fruta.

## Organização

| Arquivo | Responsabilidade |
| --- | --- |
| [principal.lep](principal.lep) | Início e ligação com o laço gráfico |
| [regras.lep](regras.lep) | Corpo, alimento, curvas, colisões, pontuação e tempo |
| [controles.lep](controles.lep) | Entrada ordenada, pausa e reinício |
| [desenho.lep](desenho.lep) | Grade, corpo, fruta, olhos e painel |
| [validar.lep](validar.lep) | Verificações sem abrir janela |

## Estratégias para comparar

1. **Lista da cabeça à cauda:** o corpo é uma lista de posições `[coluna, linha]`. Cada passo acrescenta a nova cabeça; sem crescimento, a última célula é descartada. Outra possibilidade seria usar uma fila circular para reduzir cópias.
2. **Curvas em fila:** até duas mudanças de direção podem ficar pendentes. Apertar Cima e depois Esquerda entre dois passos deve produzir duas curvas, uma por passo. A inversão é comparada com a última direção pendente, não apenas com a direção atual.
3. **Ordem dos acontecimentos:** os controles usam `ev:eventos` para preservar a ordem das teclas. Um conjunto como `ev:apertadas` informa quais teclas chegaram, mas não em qual ordem. Os testes também podem fornecer apenas o conjunto, com uma ordem de prioridade definida no código.
4. **A cauda é um caso especial:** a colisão considera se ela será removida neste passo. Verificar sempre o corpo inteiro impediria um movimento válido; removê-la antes de decidir o crescimento poderia permitir uma colisão indevida.
5. **Alimento sem tentativas repetidas:** enumeramos as células livres e sorteamos uma delas. Isso garante término mesmo quando falta uma única célula. Em tabuleiros maiores, pode-se comparar essa abordagem com a manutenção de um conjunto de posições livres.
6. **Passos com tempo acumulado:** o desenho roda a 60 quadros por segundo, mas o corpo anda em intervalos próprios. Um quadro pode avançar mais de um passo. Os controles limitam o tempo recebido a 0,25 segundo para evitar um salto enorme depois de um atraso.

O estado da partida é um mapa compartilhado; a simulação altera seus campos, e o desenho apenas os consulta. `regras::crie(largura, altura)` permite testar outras dimensões, a partir de 5×5.

## Verificar

```sh
expressa exemplos/jogos/snake-cx/validar.lep
expressa --backend ast exemplos/jogos/snake-cx/validar.lep
expressa --backend bytecode exemplos/jogos/snake-cx/validar.lep
```

As verificações cobrem espera inicial, comida fora do corpo, crescimento, pontuação, níveis, fila de curvas, inversões, colisões, entrada na cauda, tabuleiro cheio, frequência de atualização, teclado, pausa, perda de foco, reinício e desenho.

Para salvar uma prévia sem abrir janela:

```sh
expressa exemplos/jogos/snake-cx --imagem
```

O arquivo `previa.png` será criado nesta pasta. Também é possível informar um caminho depois de `--imagem`.

O programa ativa `define_opcao(:fechar_janelas, verdadeiro)`. Ao terminar, suas janelas são fechadas automaticamente, inclusive em caso de erro ou interrupção. Use uma compilação da Expressa e da Aula que inclua essa opção. O arquivo snake.lep continua disponível como entrada alternativa.
