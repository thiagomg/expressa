# AST aprovado × interpretador de bytecode

Este é o resultado histórico da primeira entrega. As experiências posteriores, medidas individualmente sem sobrescrever estes dados, estão em [BYTECODE-OPTIMIZATIONS.md](BYTECODE-OPTIMIZATIONS.md).

Comparação desktop da implementação atual, sem JIT. A referência é o AST revisado e commitado em `bd654b5`, preservado antes da migração. **Não** é a versão anterior às correções AST nem o antigo baseline isolado. O candidato é o bytecode da árvore de trabalho identificada pelos hashes abaixo.

## Método

- Host: Linux-7.3.0-10-generic-x86_64-with-glibc2.44.
- CPU: Intel(R) Core(TM) i5-10210U CPU @ 1.60GHz.
- Compilador: rustc 1.98.1 (48a229cea 2026-09-01).
- Captura: 2026-11-09T06:16:26.226050+00:00.
- 15 amostras por programa/backend após 3 rodadas de aquecimento; ordem intercalada pseudoaleatória, seed 0.
- Oito programas, 240 amostras medidas e 48 execuções de aquecimento. Todas passaram na checagem da saída exata e do código de saída.
- Mesma configuração decimal/locale, bibliotecas e build release. O runner seleciona explicitamente cada backend.
- Processos novos; os tempos incluem inicialização, parsing, compilação quando aplicável, execução e liberação. Não se compara bytecode pré-compilado com AST a partir do texto.
- CPU = tempo de usuário + sistema. RSS = pico de memória residente do processo completo, coletado pelo helper nativo; não inclui a memória do driver Python.

```sh
cargo build -p expressa --release --bin expressa --offline --locked
python3 benchmarks/run.py --engine ast=target/ast-reference-bd654b5/expressa-ast --engine bytecode=target/release/expressa --backend ast=ast --backend bytecode=bytecode --repeat 15 --warmup 3 --output benchmarks/results/ast-vs-bytecode.json
```

## CPU

Valores em ms. A variação compara as medianas; negativo significa menos CPU no bytecode. Mínimo/máximo expõem a dispersão, sem prometer significância estatística.

| Programa | AST mediana | Bytecode mediana | Variação | AST mín.–máx. | Bytecode mín.–máx. |
| --- | ---: | ---: | ---: | ---: | ---: |
| Soma em laço | 58.548 | 51.608 | -11.9% | 57.528–65.089 | 50.009–57.087 |
| Avaliação dinâmica repetida | 4.453 | 6.399 | +43.7% | 4.295–9.945 | 5.982–27.531 |
| Chamadas e closures | 67.771 | 71.723 | +5.8% | 66.317–92.112 | 70.043–78.975 |
| Listas e mapas | 59.460 | 57.738 | -2.9% | 57.307–65.339 | 57.161–65.639 |
| Alocação e ciclos | 7.685 | 7.044 | -8.3% | 7.496–8.754 | 6.913–9.381 |
| Produto de matrizes | 2.848 | 2.928 | +2.8% | 2.712–6.114 | 2.794–3.213 |
| Recursão (2.000 chamadas) | 8.680 | 3.570 | -58.9% | 8.208–20.377 | 3.384–7.498 |
| Inicialização | 1.107 | 1.125 | +1.6% | 1.016–2.192 | 1.035–1.293 |

## Memória de pico

Valores em KiB (1.024 bytes), medianas por programa. A variação é do RSS total; não mede apenas objetos da linguagem.

| Programa | AST | Bytecode | Diferença KiB | Variação |
| --- | ---: | ---: | ---: | ---: |
| Soma em laço | 4440 | 4648 | +208 | +4.7% |
| Avaliação dinâmica repetida | 4524 | 4692 | +168 | +3.7% |
| Chamadas e closures | 4480 | 4664 | +184 | +4.1% |
| Listas e mapas | 4512 | 4732 | +220 | +4.9% |
| Alocação e ciclos | 4796 | 5024 | +228 | +4.8% |
| Produto de matrizes | 4552 | 4884 | +332 | +7.3% |
| Recursão (2.000 chamadas) | 16684 | 6516 | -10168 | -60.9% |
| Inicialização | 4380 | 4668 | +288 | +6.6% |

## Interpretação

- Recursão: 2.43× mais rápida em CPU e 60.9% menos RSS de pico neste programa. Frames explícitos evitam crescer a pilha nativa por chamada Expressa.
- Soma em laço: 11.9% menos CPU neste ensaio, após reduzir instruções e alocações de atribuições de nomes.
- `avalie` repetido: 43.7% mais CPU. O bytecode acrescenta compilação por chamada e ainda não possui cache de código.
- Chamadas curtas e outros casos não apresentam ganho universal; consultar todas as linhas, inclusive regressões. Inicialização e matrizes pequenas são especialmente sensíveis a ruído.
- Fora da recursão, o bytecode usa mais RSS neste conjunto. Código adicional, compilação e estruturas da VM fazem parte do pico medido; estes dados não isolam a causa ou o heap da linguagem.
- O candidato ainda inclui o backend AST de referência. Esse fato e os diferentes layouts dos executáveis devem ser considerados ao interpretar pequenos deltas de RSS.
- Não se calculou uma média global de speedups. Os resultados dependem do programa e não provam desempenho ou compatibilidade no Pico 2.

## Proveniência e dados

- Fonte AST: commit `bd654b597e873d1cf266636a88ecc98fad54db5e`.
- Binário AST SHA-256: `32ff26b98c199b3baa906f42da8de997d35e2cb7cb44b85cc53c855c827aa5f7`; 4069736 bytes.
- Binário bytecode SHA-256: `d8e25a0712d8084c6b57bbfb51c9982eee6658d71b62912f155a4b0d019b7f3f`; 4212760 bytes.
- Árvore Rust/fontes nativas do candidato SHA-256: `515715d1b3d044cf8f1ec35b7b8423d0c23f06499bcde97be546e307ebca2047`.
- [Amostras finais e metadados](results/ast-vs-bytecode.json): CPU de usuário/sistema, tempo decorrido, utilização média, RSS, hashes das entradas, binários e ferramentas.
- [Manifesto da referência](references/ast-bd654b5.json). Binário e fonte aprovados preservados em `target/ast-reference-bd654b5/`; são artefatos locais, não novos commits.
- [Primeira medição exploratória](results/ast-vs-bytecode-initial.json), antes das otimizações de despacho. O binário inicial e um overlay das fontes foram preservados em `target/bytecode-initial/`. Ela não substitui os resultados finais.
- [Protocolo e limitações](README.md). Flash, SRAM, heap vivo, pilhas, latência de cancelamento e desempenho na placa ainda precisam de instrumentação/hardware.
