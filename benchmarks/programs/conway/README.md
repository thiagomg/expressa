# Conway gráfico determinístico

Exemplo para comparar simulação, construção/rasterização do desenho e o caminho completo. Sem números aleatórios, entrada interativa ou espera artificial. A regra é B3/S23, com bordas conectadas: o vizinho além de uma borda está no lado oposto. Cada bloco inicial de 16×16 contém um bloco estável, um oscilador e um planador.

A subpasta é intencional: `benchmarks/run.py` seleciona apenas os `.lep` diretamente em `programs/`, exige a saída `verdadeiro` e executa casos curtos. Este exemplo é executado explicitamente; não altera os resultados nem a seleção daquela suíte.

## Execução

```sh
# Argumentos: modo lado geracoes aquecimento saida
# Padrão: simulacao 32 1000 100 janela
expressa benchmarks/programs/conway/conway.lep simulacao 32 1000 100
expressa benchmarks/programs/conway/conway.lep desenho 32 1000 100 janela
expressa benchmarks/programs/conway/conway.lep completo 32 1000 100 janela

# Desenhar/rasterizar sem abrir janela (inclusive em máquinas sem display):
expressa benchmarks/programs/conway/conway.lep completo 32 20 5 raster

# Selecionar explicitamente o executor, no mesmo build completo:
expressa --backend ast benchmarks/programs/conway/conway.lep simulacao 32 20 5
expressa --backend bytecode benchmarks/programs/conway/conway.lep simulacao 32 20 5
```

Substitua `expressa` por `target/release/expressa` no checkout local. Lados aceitos: 32, 64 e 128. Gerações: inteiros de 1 a 1000. Aquecimento: inteiros de 0 a 1000. Células de 4×4 pixels, até uma janela de 512×512. Argumentos inválidos encerram com erro.

| Modo | Aquecimento | Fase de execução |
| --- | --- | --- |
| `simulacao` | Calcula gerações | Calcula gerações sem carregar a biblioteca gráfica |
| `desenho` | Calcula e desenha gerações | Desenha a sequência pré-calculada na fase `preparacao` |
| `completo` | Calcula e desenha gerações | Calcula e desenha cada geração |

Os modos gráficos percorrem todas as células e emitem um retângulo por célula viva, limpando os comandos anteriores a cada quadro. `janela` chama `mostre`; `raster` chama `como_imagem`, exercitando desenho e criação da imagem em memória, sem IPC/viewer. Esses destinos têm custos diferentes e devem ser registrados separadamente. O modo simulação funciona também no perfil mínimo. Os demais exigem `graphics`.

A linguagem permite imports no nível de módulo. Para carregar gráficos somente quando necessário, a preparação dos modos gráficos faz um único `avalie` que importa `tela_grafica`. Não há avaliação dinâmica dentro do laço de gerações/desenho.

## Correção e memória

A última linha contém modo, lado, gerações, aquecimento e `[celulas_vivas, checksum]`. O checksum percorre a matriz por linhas, aplicando `(hash * 31 + celula) % 1000000007`, iniciado em zero. É uma verificação simples de regressão, não um hash criptográfico. O resultado usa o locale numérico da execução.

Para os mesmos parâmetros, os três modos e os dois executores devem produzir o mesmo par final, depois de `aquecimento + geracoes` passos. Referências conferidas com uma implementação independente em Python:

| Lado | Gerações | Aquecimento | Vivas | Checksum |
| --- | ---: | ---: | ---: | ---: |
| 32 | 1 | 0 | 48 | 632510400 |
| 32 | 5 | 3 | 48 | 751502599 |
| 64 | 2 | 1 | 192 | 117119233 |
| 128 | 1 | 0 | 768 | 954862116 |

`desenho` guarda uma matriz nova por geração medida. Isso evita recalcular a simulação no laço de desenho, mas aumenta o pico de memória: só as células de 1000 quadros de 128×128 ocupam aproximadamente 125 MiB, além de cabeçalhos, listas, temporários, imagens e runtime. Comece com 32×32 e poucas gerações. A adição à lista de quadros também faz parte da preparação. Não interpretar esse RSS como o consumo da simulação isolada.

## O que ainda não está medido

As linhas `fase aquecimento`, `fase preparacao` (somente desenho) e `fase execucao` identificam as etapas; o exemplo ainda não calcula tempos nem P95/P99. A impressão do resultado e seu checksum acontecem depois da fase de execução.

Uma ferramenta externa que mede o processo inteiro inclui parsing, compilação, aquecimento, preparação, checksum e encerramento. Isso não isola a fase de desenho, mesmo no modo `desenho`. O exemplo não produz automaticamente métricas de desempenho ou FPS. As medições externas de processo completo estão no [relatório AST × bytecode](../../CONWAY.md).

Para medir frames gráficos de verdade, seria útil acrescentar uma confirmação de renderização/apresentação ao protocolo do viewer, com identificador do quadro. `mostre()` não espera a apresentação; tempo de envio não é tempo de frame visível. Também será necessário um relógio monotônico de alta resolução na instrumentação para registrar atualização, envio e renderização separadamente. Essa evolução da API/instrumentação fica para revisão; o exemplo não modifica o runtime.
