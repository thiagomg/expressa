# Reteste da etapa 03 com o laptop sem uso pelo usuário

O usuário informou que deixaria o laptop sem uso até o término. As duas rodadas abaixo foram executadas sem compilações ou suítes de testes concorrentes do agente. Nenhum código do interpretador ou teste foi alterado neste reteste; não foram criados commits.

## Referências e método

- `vm02`: antes da otimização de argumentos, snapshot `02-inline-dispatch`.
- `vm03c`: depois da otimização, snapshot `03c-stack-call-path`, ainda com o parser anterior.
- `current`: commit `24b11fb`, incluindo a correção de parsing da tarefa 019.
- Os binários preservados foram conferidos por SHA-256. Entre VM02 e VM03C, o único arquivo fonte de runtime diferente é `execute.rs`. Entre VM03C e a versão atual, é `parser.rs`. A comparação isolada da otimização é VM03C × VM02; a versão atual é apresentada separadamente.
- Duas rodadas, seeds 10 e 11, 51 amostras por workload/motor após cinco aquecimentos. Nove workloads e três motores: **2.754 amostras medidas e 270 execuções de aquecimento**, todas com saída correta e sem erros.
- Mesma configuração decimal/locale, release e afinidade no núcleo 0; ordem intercalada pseudoaleatória. Processos novos, incluindo parsing, compilação, execução e limpeza.
- CPU = usuário+sistema. RSS = pico do processo inteiro. Não há contagem de alocações, heap isolado ou medição no Pico.
- O usuário não usar o laptop reduz uma fonte de interferência, mas não garante isolamento do núcleo, frequência constante ou ausência de processos do sistema. Não foram alteradas configurações de energia ou permissões do kernel.

- Rodada seed 10: captura UTC 2026-11-10T06:55:52.356724+00:00.
- Rodada seed 11: captura UTC 2026-11-10T06:58:13.060487+00:00.

## CPU: otimização isolada

Medianas em ms. Variação = VM03C / VM02 − 1; negativo significa menos CPU. Não se comparam tempos absolutos entre rodadas.

| Programa | VM02, seed 10 | VM03C, seed 10 | Variação | VM02, seed 11 | VM03C, seed 11 | Variação |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Chamadas com 0/8 argumentos | 205.717 | 202.087 | -1.8% | 202.594 | 199.192 | -1.7% |
| Soma em laço | 97.704 | 95.367 | -2.4% | 92.513 | 94.164 | +1.8% |
| Avaliação dinâmica | 12.117 | 12.046 | -0.6% | 11.837 | 11.788 | -0.4% |
| Chamadas e closures | 138.693 | 134.957 | -2.7% | 134.946 | 129.810 | -3.8% |
| Listas e mapas | 109.937 | 105.436 | -4.1% | 108.496 | 103.081 | -5.0% |
| Alocação e ciclos | 13.528 | 13.401 | -0.9% | 13.452 | 13.084 | -2.7% |
| Matrizes | 5.560 | 5.548 | -0.2% | 5.497 | 5.416 | -1.5% |
| Recursão | 6.440 | 6.244 | -3.0% | 6.290 | 6.124 | -2.6% |
| Inicialização | 1.933 | 1.908 | -1.3% | 1.872 | 1.850 | -1.2% |

Closures melhoraram 2,7–3,8%, coleções 4,1–5,0%, chamadas com 0/8 argumentos aproximadamente 1,7% e recursão 2,6–3,0%. O laço aritmético mudou de sentido entre rodadas (−2,4% e +1,8%); não há evidência de ganho consistente nesse caso. Inicialização e matrizes são curtas e sensíveis a ruído. Não se calcula um ganho global nem se afirma significância estatística.

### Dispersão dos resultados pareados por rodada

Cada posição nas listas de amostras corresponde à mesma rodada do runner. A tabela mostra a mediana e o intervalo interquartil das variações de CPU VM03C/VM02 dessas rodadas, não um intervalo de confiança. Os motores não necessariamente executam consecutivamente dentro da rodada.

| Programa | Seed 10: mediana [P25; P75] | Seed 11: mediana [P25; P75] |
| --- | ---: | ---: |
| Chamadas com 0/8 argumentos | -1.8% [-2.7%; -0.7%] | -2.5% [-3.5%; -1.3%] |
| Soma em laço | -2.8% [-3.7%; -1.4%] | -2.7% [-3.3%; -0.4%] |
| Avaliação dinâmica | -0.9% [-3.0%; +1.5%] | -1.0% [-3.7%; +1.8%] |
| Chamadas e closures | -2.8% [-4.1%; -1.5%] | -2.8% [-3.8%; -1.4%] |
| Listas e mapas | -4.2% [-5.1%; -3.4%] | -4.1% [-5.3%; -2.6%] |
| Alocação e ciclos | -1.3% [-2.9%; +0.8%] | -1.9% [-4.3%; +0.3%] |
| Matrizes | -0.3% [-1.7%; +2.3%] | -0.9% [-3.3%; +0.7%] |
| Recursão | -3.5% [-5.2%; -1.0%] | -3.0% [-4.4%; -0.2%] |
| Inicialização | -1.7% [-4.0%; +0.7%] | -0.5% [-5.3%; +2.6%] |

## Memória: otimização isolada

Medianas de RSS em KiB, incluindo código nativo, bibliotecas e pilhas.

| Programa | VM02, seed 10 | VM03C, seed 10 | Diferença | VM02, seed 11 | VM03C, seed 11 | Diferença |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Chamadas com 0/8 argumentos | 4676 | 4640 | -36 | 4676 | 4676 | +0 |
| Soma em laço | 4672 | 4688 | +16 | 4660 | 4676 | +16 |
| Avaliação dinâmica | 4644 | 4676 | +32 | 4636 | 4628 | -8 |
| Chamadas e closures | 4692 | 4676 | -16 | 4676 | 4672 | -4 |
| Listas e mapas | 4740 | 4788 | +48 | 4792 | 4792 | +0 |
| Alocação e ciclos | 5028 | 5056 | +28 | 5032 | 5020 | -12 |
| Matrizes | 4916 | 4940 | +24 | 4928 | 4936 | +8 |
| Recursão | 6448 | 6468 | +20 | 6468 | 6468 | +0 |
| Inicialização | 4672 | 4640 | -32 | 4644 | 4676 | +32 |

Os deltas ficaram entre −36 e +48 KiB na primeira rodada e entre −12 e +32 KiB na segunda. São menores que vários deltas anteriores, mas também não demonstram economia consistente de RSS. Eliminar um vetor temporário por chamada com argumentos não implica reduzir o pico de memória residente do processo.

## Versão atual, com a correção do parser

Comparação adicional `current` × VM03C, no mesmo ensaio. Essa diferença não deve ser atribuída à otimização de argumentos, presente nos dois motores.

| Programa | CPU seed 10 | CPU seed 11 | RSS Δ KiB seed 10 | RSS Δ KiB seed 11 |
| --- | ---: | ---: | ---: | ---: |
| Chamadas com 0/8 argumentos | +1.0% | +0.6% | +192 | +132 |
| Soma em laço | -0.7% | -2.0% | +168 | +164 |
| Avaliação dinâmica | -2.0% | -2.6% | +192 | +180 |
| Chamadas e closures | +0.6% | +2.3% | +140 | +152 |
| Listas e mapas | +1.4% | +2.6% | +168 | +92 |
| Alocação e ciclos | +1.6% | +3.1% | +152 | +144 |
| Matrizes | -2.3% | -0.8% | +164 | +112 |
| Recursão | -1.3% | -0.8% | +268 | +156 |
| Inicialização | -6.3% | -5.5% | +216 | +112 |

A versão atual apresentou RSS 92–268 KiB maior que VM03C nesses ensaios. Mudanças no parser e no layout do executável fazem parte da comparação; não foi isolada a causa desse aumento e não se conclui que ele corresponda ao heap do parser.

## Testes funcionais atuais

- Backend bytecode em debug: **399 testes passaram** (365 biblioteca, 21 regressões, 13 diferenciais/integração).
- Backend AST selecionado em debug: **399 testes passaram**. Os testes diferenciais continuam exercitando ambos os motores explicitamente.
- Release: **399 testes passaram na repetição sem outra suíte concorrente**.
- Na primeira execução release, concorrente com a suíte debug, dois testes falharam: `discarded_process_values_keep_the_ast_cleanup_timing` e `stack_arguments_release_processes_on_return_and_arity_error`. Em ambos, um arquivo esperado após `durma(0.2)` não existia no caso em que o processo deveria continuar vivo.
- A suíte release inteira passou ao ser repetida sem a outra suíte. Depois, os dois testes de processos passaram em **três repetições isoladas**, com `--test-threads=1` (seis execuções de teste).

Essas falhas foram intermitentes neste reteste e são compatíveis com sensibilidade ao agendamento dos subprocessos na janela de 200 ms. Isso não prova que o agendamento seja sua única causa. Os testes temporizados merecem sincronização mais determinística em uma mudança separada. Não foram modificados, ignorados nem removidos neste reteste. A falha inicial é registrada mesmo com as repetições passando.

Os testes funcionais começaram somente após o término das duas rodadas de benchmark; portanto, a execução concorrente inicial dessas suítes não interfere nos números de desempenho. A suíte gráfica do workspace não foi repetida. Dois testes existentes da biblioteca exigiram acesso fora do sandbox para temporários em `~/.cache`.

```sh
cargo test -p expressa --offline --locked --lib --test ast_regressions --test bytecode_conformance --quiet
cargo test -p expressa --release --offline --locked --lib --test ast_regressions --test bytecode_conformance --quiet
EXPRESSA_BACKEND=ast cargo test -p expressa --offline --locked --lib --test ast_regressions --test bytecode_conformance --quiet
cargo test -p expressa --release --offline --locked --test bytecode_conformance process --quiet -- --test-threads=1
```

## Dados e reprodução

- [Rodada seed 10](results/bytecode-step-03-idle-retest-10.json): todas as amostras, tempos de usuário/sistema/parede, RSS e metadados.
- [Rodada seed 11](results/bytecode-step-03-idle-retest-11.json).
- [Manifesto das referências e da versão atual](references/bytecode-step-03-idle-retest.json): hashes, commit e protocolo. A versão atual e seu arquivo de fontes foram preservados em `target/bytecode-steps/03-idle-retest-current/`.
- [Resultados anteriores](BYTECODE-STEP-03.md) preservados integralmente.

```sh
cargo build -p expressa --release --bin expressa --offline --locked
taskset -c 0 python3 benchmarks/run.py --engine vm02=target/bytecode-steps/02-inline-dispatch/expressa --engine vm03c=target/bytecode-steps/03c-stack-call-path/expressa --engine current=target/bytecode-steps/03-idle-retest-current/expressa --backend vm02=bytecode --backend vm03c=bytecode --backend current=bytecode --repeat 51 --warmup 5 --seed 10 --output benchmarks/results/bytecode-step-03-idle-retest-10.json
```

A segunda rodada usa `--seed 11` e outro arquivo de saída.
