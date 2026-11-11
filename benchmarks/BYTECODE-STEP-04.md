# Etapa 04: parâmetros por posição e testes sincronizados

Duas mudanças independentes foram implementadas para revisão, sem commits automáticos: [testes de subprocessos](../lang-tasks/044-deterministic-process-tests.md) e [parâmetros por posição](../lang-tasks/045-indexed-function-parameters.md). As medições comparam o mesmo runtime antes/depois da otimização; a mudança de testes não faz parte do executável release.

## Implementação

- `LoadLocal`/`StoreLocal` acessam diretamente parâmetros no escopo principal da função. Blocos, laços e nomes não pertencentes aos parâmetros continuam por nome, preservando sombreamento e nomes introduzidos depois.
- Cada função compilada compartilha um mapa nome→posição entre suas chamadas. Cada chamada possui seu próprio vetor canônico de valores, sem recriar as chaves dos parâmetros. Não há uma cópia extra dos parâmetros no mapa de variáveis.
- `avalie`, closures, callbacks nativos, depurador e coletor acessam esse mesmo armazenamento. Os testes confirmam alterações por avaliação dinâmica, barreiras de escrita entre funções, aliases e ciclos envolvendo parâmetros.
- `Env.vars` usa `Bindings`, que combina o mapa dos outros nomes com o vetor dos parâmetros. Isso muda um tipo concreto acessível indiretamente aos hooks Rust; código externo que exige HashMap ou seus métodos específicos precisa de ajuste. [Compatibilidade de embedding](../docs/embedding.md). Não há mudança de sintaxe, escopo ou modelo numérico.
- Os testes de retenção de processos usam prontidão, autorização/resposta e EOF por socket Unix, com prazo apenas no controlador. Os dois testes foram movidos para a biblioteca para usar checkpoints internos com backend explícito. O auxiliar marcado `ignore` é executado explicitamente pelos testes; as regressões não foram ignoradas.

## Método e proveniência

- Referência: versão atual do repositório antes de alterar armazenamento e instruções, preservada em `04-before-parameter-slots`. Não se reutiliza o antigo AST aprovado como denominador desta otimização.
- Candidato medido: `04-parameter-slots`.
- Dois ensaios bytecode: 51 amostras por workload/motor após cinco aquecimentos; seeds 20 e 21; nove workloads e dois motores. São **1.836 amostras medidas e 180 aquecimentos**.
- Mesmo build release/offline/locked, decimal, locale e afinidade no núcleo 0, com ordem intercalada. Nenhuma compilação ou suíte do agente foi executada durante a coleta de amostras. Não houve controle de frequência, temperatura ou de todos os processos do host.
- CPU = usuário+sistema; RSS = pico do processo inteiro. Os tempos incluem parsing, compilação, execução e limpeza. Não há medição isolada de heap/alocador ou hardware Pico.
- Todas as amostras e aquecimentos passaram na verificação da saída exata e sucesso do processo. Os dados anteriores foram preservados.
- Depois da medição, somente o auxiliar `cfg(test)` foi refinado para não encerrar por temporizador. Fontes finais e testes estão em `04-parameter-slots-reviewed-tests`; seu executável possui **exatamente o mesmo SHA-256** do candidato medido. O manifesto distingue os fingerprints de fontes e arquivos de cada momento.

## CPU do bytecode

Medianas em ms; negativo na variação significa menos CPU. Cada delta usa motores executados no mesmo ensaio.

| Programa | Antes, seed 20 | Slots, seed 20 | Variação | Antes, seed 21 | Slots, seed 21 | Variação |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Chamadas com 0/8 argumentos | 197.835 | 125.702 | -36.5% | 203.646 | 128.983 | -36.7% |
| Soma em laço | 93.706 | 97.106 | +3.6% | 94.396 | 100.508 | +6.5% |
| Avaliação dinâmica | 11.741 | 11.222 | -4.4% | 11.764 | 11.367 | -3.4% |
| Chamadas e closures | 132.161 | 120.876 | -8.5% | 135.607 | 123.644 | -8.8% |
| Listas e mapas | 104.711 | 104.394 | -0.3% | 106.510 | 107.721 | +1.1% |
| Alocação e ciclos | 13.556 | 13.316 | -1.8% | 13.652 | 13.561 | -0.7% |
| Matrizes | 5.497 | 5.365 | -2.4% | 5.541 | 5.509 | -0.6% |
| Recursão | 6.252 | 5.554 | -11.2% | 6.279 | 5.712 | -9.0% |
| Inicialização | 1.891 | 1.754 | -7.2% | 1.916 | 1.869 | -2.5% |

Os ganhos mais claros são aproximadamente **36–37% menos CPU** no teste de chamadas com 0/8 argumentos (1,57–1,58× de speedup), **8,5–8,8% em closures** e **9,0–11,2% em recursão**. Há uma regressão consistente de **3,6–6,5% no laço aritmético**. Coleções ficaram próximas (−0,3% e +1,2%). Os casos curtos não sustentam conclusões fortes. Não se calcula uma média global de ganhos.

Dispersão de CPU no segundo ensaio, mínimo–máximo em ms:

| Programa | Antes | Slots |
| --- | ---: | ---: |
| Chamadas com 0/8 argumentos | 97.787–304.767 | 61.849–143.015 |
| Soma em laço | 44.837–101.291 | 47.690–111.327 |
| Avaliação dinâmica | 5.589–18.048 | 5.401–21.455 |
| Chamadas e closures | 65.628–148.710 | 59.185–137.796 |
| Listas e mapas | 50.340–116.604 | 52.430–115.247 |
| Alocação e ciclos | 6.478–24.235 | 6.458–16.690 |
| Matrizes | 2.700–8.827 | 2.740–8.499 |
| Recursão | 3.166–7.734 | 2.846–7.689 |
| Inicialização | 0.949–5.162 | 0.964–2.796 |

## RSS do bytecode

Medianas em KiB; a diferença é candidato − referência.

| Programa | Antes, seed 20 | Slots, seed 20 | Diferença | Antes, seed 21 | Slots, seed 21 | Diferença |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Chamadas com 0/8 argumentos | 4588 | 4776 | +188 | 4680 | 4676 | -4 |
| Soma em laço | 4612 | 4816 | +204 | 4692 | 4628 | -64 |
| Avaliação dinâmica | 4600 | 4816 | +216 | 4628 | 4668 | +40 |
| Chamadas e closures | 4608 | 4808 | +200 | 4680 | 4664 | -16 |
| Listas e mapas | 4692 | 4936 | +244 | 4788 | 4756 | -32 |
| Alocação e ciclos | 4976 | 5192 | +216 | 5056 | 5008 | -48 |
| Matrizes | 4836 | 5088 | +252 | 4948 | 4860 | -88 |
| Recursão | 6336 | 6424 | +88 | 6472 | 6248 | -224 |
| Inicialização | 4608 | 4792 | +184 | 4696 | 4668 | -28 |

Na primeira rodada, o RSS aumentou 88–252 KiB; na segunda, variou de −224 a +40 KiB. A inversão com os mesmos binários impede afirmar redução consistente de memória residente. O armazenamento por chamada evita recriar as chaves dos parâmetros, mas a estrutura de ambiente e o código também mudam. Não foi isolada a causa dos deltas de RSS e não se atribui todo o efeito a heap ou layout do executável.

O executável passou de 4221944 para 4229824 bytes (+7880 bytes). Esse tamanho não representa firmware Pico.

## Efeito no AST

O armazenamento de bindings também é usado pelo AST. Foi feito um ensaio exploratório antes/depois com backend AST selecionado explicitamente: 31 amostras, três aquecimentos, seed 22, mesma afinidade (558 amostras medidas e 54 aquecimentos). Todas passaram. O AST continua armazenando seus parâmetros pelo caminho nomeado; não executa os opcodes locais.

**Não se compara diretamente CPU AST × bytecode dividindo números desses ensaios separados.** As condições temporais do host variaram e a comparação válida aqui é antes/depois do mesmo motor em cada ensaio.

| Programa | AST antes CPU ms | AST depois CPU ms | Variação CPU | RSS Δ KiB |
| --- | ---: | ---: | ---: | ---: |
| Chamadas com 0/8 argumentos | 115.090 | 119.722 | +4.0% | -52 |
| Soma em laço | 58.745 | 61.677 | +5.0% | -80 |
| Avaliação dinâmica | 4.431 | 4.482 | +1.2% | -80 |
| Chamadas e closures | 68.452 | 69.357 | +1.3% | -180 |
| Listas e mapas | 59.366 | 60.074 | +1.2% | -44 |
| Alocação e ciclos | 7.540 | 7.675 | +1.8% | -84 |
| Matrizes | 2.868 | 2.794 | -2.6% | -132 |
| Recursão | 8.640 | 8.638 | -0.0% | +44 |
| Inicialização | 1.041 | 1.009 | -3.1% | -132 |

Esse ensaio mostrou cerca de +5% de CPU em aritmética AST e +4% no teste de argumentos AST. É um resultado exploratório sem segunda rodada. A mudança não é neutra ao motor de referência. A maior estrutura de ambiente, a verificação do caminho de parâmetros e o código nativo gerado são hipóteses a investigar; não foi demonstrada a contribuição individual de cada uma.

## Validação e decisão

- **402 testes do núcleo passaram** em debug, release e com o backend AST selecionado: 369 da biblioteca, 21 regressões e 12 diferenciais/integração. Há ainda um auxiliar de subprocesso intencionalmente ignorado no harness principal e executado explicitamente pelos testes.
- **Workspace completo: 605 testes passaram**, quatro marcados ignore (um auxiliar novo e três existentes), nenhum teste filtrado. Os processos auxiliares iniciados pelos testes não foram contados duas vezes.
- Os dois testes sincronizados passaram em cinco repetições debug e cinco release, além das suítes concorrentes. Nenhuma espera fixa de 200 ms é usada para decidir sucesso.
- `cargo clippy --workspace --all-targets --offline --locked -- -D warnings`, formatação e verificação de whitespace passaram.
- Os testes usam sockets Unix locais e alguns testes existentes usam `~/.cache`; foram executados fora do sandbox quando necessário. A versão bloqueada pelo sandbox não foi tratada como falha semântica.

**Mantida para revisão**, com os ganhos em chamadas e a regressão aritmética explícitos. Não é uma melhora universal nem uma prova de adequação ao Pico. Antes de ampliar slots para outros nomes/escopos, convém investigar a regressão no caminho nomeado e medir heap/alocações separadamente. Nenhuma mudança adicional foi agrupada para esconder esse custo.

## Artefatos e reprodução

- [Tarefa dos testes determinísticos](../lang-tasks/044-deterministic-process-tests.md) e [patch independente](bytecode-steps/04-process-handshake.patch).
- [Tarefa dos parâmetros](../lang-tasks/045-indexed-function-parameters.md) e [patch independente](bytecode-steps/04-parameter-slots.patch).
- [Manifesto e hashes](references/bytecode-step-04.json). Binários e fontes estão preservados localmente em `target/bytecode-steps/04-*/`.
- [Seed 20](results/bytecode-step-04-20.json), [seed 21](results/bytecode-step-04-21.json) e [efeito no AST](results/bytecode-step-04-ast-impact.json): amostras individuais de CPU, parede e RSS, mediana de cada métrica, hashes de entradas e ferramentas, plataforma e compilador.

```sh
cargo build -p expressa --release --bin expressa --offline --locked
taskset -c 0 python3 benchmarks/run.py --engine before=target/bytecode-steps/04-before-parameter-slots/expressa --engine slots=target/bytecode-steps/04-parameter-slots/expressa --backend before=bytecode --backend slots=bytecode --repeat 51 --warmup 5 --seed 20 --output benchmarks/results/bytecode-step-04-20.json
```

A confirmação usa seed 21 e arquivo de saída separado. O ensaio AST usa os mesmos executáveis (o snapshot final tem hash idêntico ao candidato), backends `ast`, 31 amostras, três aquecimentos e seed 22. Nenhum resultado antigo foi sobrescrito.
