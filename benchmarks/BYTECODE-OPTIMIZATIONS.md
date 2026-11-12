# Otimizações incrementais da VM

Cada experiência altera um ponto do runtime, preserva o binário e as fontes anteriores e mede novamente os mesmos programas. Não há commits automáticos. Ganhos, regressões e experiências descartadas ficam registrados. As medições anteriores em [AST-VS-BYTECODE.md](AST-VS-BYTECODE.md) são históricas; os deltas de cada etapa usam referências executadas novamente no mesmo ensaio.

## Protocolo

1. Preservar fontes, binário release e hashes antes da alteração.
2. Implementar uma hipótese isolada, sem mudar a semântica da linguagem.
3. Executar os testes do núcleo, incluindo equivalência AST/VM.
4. Compilar em release com o mesmo comando e sem alterar dependências ou configuração numérica.
5. Intercalar referência e candidato no runner, verificando todas as saídas. Medir CPU, tempo decorrido e RSS; guardar amostras individuais e dispersão.
6. Repetir resultados ambíguos. Manter ou descartar a experiência com justificativa. Somente uma mudança aceita vira a referência da próxima etapa.

Os binários/fontes locais estão em `target/bytecode-steps/`. Os hashes estão no [manifesto](references/bytecode-steps.json). As diferenças de código de cada experiência estão em [bytecode-steps/](bytecode-steps/), inclusive as descartadas. O snapshot `00-baseline` corresponde exatamente ao binário e às fontes identificados no resultado histórico `ast-vs-bytecode.json`; os dois hashes foram conferidos antes do início.

As tabelas abaixo comparam medianas de CPU (usuário+sistema) e RSS máximo do processo inteiro. Negativo significa redução. RSS inclui bibliotecas, código e pilhas; não é uma medida de heap ou SRAM do Pico. O layout dos executáveis e o ambiente podem influenciar os resultados. Não foi instrumentada a contagem de alocações. `perf` não está disponível por restrição de acesso aos contadores do host. Os tempos absolutos também variaram em relação ao ensaio histórico; não se calculam ganhos misturando execuções de momentos diferentes.

## Experiência 01: escopo inicial dentro do frame

Hipótese: guardar o primeiro escopo diretamente no frame e alocar o vetor apenas para escopos adicionais evita uma alocação por chamada comum. Isso aumenta o frame e acrescenta uma escolha ao acessar o escopo atual.

**Decisão: descartada.** O RSS aumentou nos dois ensaios e os resultados de CPU foram mistos. Em closures, o primeiro ensaio piorou e o segundo melhorou; no laço aritmético, o segundo piorou. Não há evidência de benefício geral que justifique manter essa estrutura. A alteração foi retirada do runtime; seu patch, fontes, binário e dados foram preservados.

Foram executadas 21 amostras por programa/motor, após 3 aquecimentos, em ordem intercalada. O primeiro ensaio incluiu AST, VM original e candidato (504 amostras medidas); a confirmação comparou as duas VMs com seed 1 (336 amostras). A afinidade original incluía os núcleos 0–7. Os 394 testes do núcleo passaram antes da medição.

Dados: [primeiro ensaio](results/bytecode-step-01.json), [confirmação](results/bytecode-step-01-confirmation.json), [patch descartado](bytecode-steps/01-inline-root.patch).

| Programa | CPU antes/depois (ms), ensaio 1 | Variação | CPU antes/depois (ms), confirmação | Variação |
| --- | ---: | ---: | ---: | ---: |
| Soma em laço | 107.052 / 107.645 | +0.6% | 104.880 / 115.626 | +10.2% |
| Avaliação dinâmica | 13.381 / 12.278 | -8.2% | 12.853 / 12.302 | -4.3% |
| Chamadas e closures | 149.994 / 158.885 | +5.9% | 162.107 / 149.574 | -7.7% |
| Listas e mapas | 124.145 / 125.656 | +1.2% | 125.500 / 124.863 | -0.5% |
| Alocação e ciclos | 14.164 / 13.914 | -1.8% | 14.295 / 13.984 | -2.2% |
| Matrizes | 5.735 / 5.767 | +0.6% | 5.654 / 5.799 | +2.6% |
| Recursão | 6.937 / 6.692 | -3.5% | 6.998 / 6.688 | -4.4% |
| Inicialização | 2.010 / 1.847 | -8.1% | 2.011 / 1.860 | -7.5% |

| Programa | RSS antes/depois (KiB), ensaio 1 | Diferença | RSS antes/depois (KiB), confirmação | Diferença |
| --- | ---: | ---: | ---: | ---: |
| Soma em laço | 4540 / 4888 | +348 | 4664 / 4872 | +208 |
| Avaliação dinâmica | 4548 / 4884 | +336 | 4628 / 4888 | +260 |
| Chamadas e closures | 4552 / 4888 | +336 | 4668 / 4848 | +180 |
| Listas e mapas | 4644 / 4992 | +348 | 4760 / 4980 | +220 |
| Alocação e ciclos | 4896 / 5200 | +304 | 4988 / 5252 | +264 |
| Matrizes | 4816 / 5140 | +324 | 4984 / 5144 | +160 |
| Recursão | 6304 / 6700 | +396 | 6468 / 6736 | +268 |
| Inicialização | 4516 / 4888 | +372 | 4660 / 4864 | +204 |

## Experiência 02: despacho incorporado ao laço principal

Hipótese: `#[inline(always)]` em `Machine::step` permite ao compilador incorporar o despacho ao laço de execução e otimizar a passagem de `Result<Step, EvalError>` entre as duas funções. É uma indicação ao compilador Rust, sem alteração das instruções ou da linguagem. O efeito precisa ser reavaliado quando o compilador ou o alvo mudar.

Esta experiência parte de `00-baseline`, porque a primeira foi descartada. O ensaio usa afinidade fixada no núcleo 0, 31 amostras por programa/motor e 3 aquecimentos, com AST, VM original e candidato intercalados (744 amostras medidas). Uma confirmação com seed 1 comparou as duas VMs com a mesma afinidade, 31 amostras e 3 aquecimentos (496 amostras medidas). Todas as saídas foram validadas nos dois ensaios.

**Decisão: mantida no desktop para revisão.** As medianas de CPU diminuíram nos dois ensaios, com ganhos de aproximadamente 4–5% no laço aritmético, 5–6% em closures e 8% em coleções. Inicialização e matrizes são muito curtas para sustentar conclusões fortes. Não há teste de significância estatística nem promessa de ganho em qualquer programa.

**Não houve melhora de memória.** O RSS aumentou 168–316 KiB no primeiro ensaio e 36–60 KiB na confirmação. A variação entre ensaios, sem mudar os binários, impede tratar o primeiro delta como um custo fixo da alteração. O código não acrescenta estruturas de heap, mas inlining altera o código nativo e pode mudar o uso da pilha e o layout do executável. Não foi isolada a causa do RSS. O alvo Pico precisa de avaliação própria; esta decisão desktop não valida SRAM ou flash na placa.

O executável passou de 4212760 para 4216704 bytes (+3944 bytes). Esse tamanho não é o tamanho de um firmware Pico. A inspeção dos símbolos com `nm -S` confirmou que o despacho separado de `Machine::step` foi incorporado em `Machine::run` no candidato release.

| Programa | CPU antes/depois (ms), ensaio 1 | Variação | CPU antes/depois (ms), confirmação | Variação |
| --- | ---: | ---: | ---: | ---: |
| Soma em laço | 104.691 / 100.152 | -4.3% | 104.038 / 98.551 | -5.3% |
| Avaliação dinâmica | 12.930 / 12.087 | -6.5% | 12.575 / 12.125 | -3.6% |
| Chamadas e closures | 149.375 / 141.764 | -5.1% | 147.644 / 139.061 | -5.8% |
| Listas e mapas | 120.647 / 111.129 | -7.9% | 119.629 / 110.415 | -7.7% |
| Alocação e ciclos | 14.612 / 13.555 | -7.2% | 14.166 / 13.483 | -4.8% |
| Matrizes | 5.634 / 5.521 | -2.0% | 5.546 / 5.511 | -0.6% |
| Recursão | 6.867 / 6.469 | -5.8% | 6.823 / 6.340 | -7.1% |
| Inicialização | 1.937 / 1.792 | -7.5% | 1.904 / 1.843 | -3.2% |

| Programa | RSS antes/depois (KiB), ensaio 1 | Diferença | RSS antes/depois (KiB), confirmação | Diferença |
| --- | ---: | ---: | ---: | ---: |
| Soma em laço | 4660 / 4828 | +168 | 4668 / 4708 | +40 |
| Avaliação dinâmica | 4668 / 4888 | +220 | 4684 / 4720 | +36 |
| Chamadas e closures | 4676 / 4876 | +200 | 4664 / 4724 | +60 |
| Listas e mapas | 4760 / 4940 | +180 | 4768 / 4824 | +56 |
| Alocação e ciclos | 5020 / 5196 | +176 | 5028 / 5068 | +40 |
| Matrizes | 4928 / 5104 | +176 | 4904 / 4964 | +60 |
| Recursão | 6460 / 6776 | +316 | 6460 / 6512 | +52 |
| Inicialização | 4628 / 4864 | +236 | 4660 / 4708 | +48 |

Dispersão de CPU na confirmação (mínimo–máximo, ms):

| Programa | VM anterior | Candidato |
| --- | ---: | ---: |
| Soma em laço | 92.239–144.270 | 87.730–106.615 |
| Avaliação dinâmica | 11.685–14.139 | 10.747–13.589 |
| Chamadas e closures | 130.518–178.481 | 125.884–155.777 |
| Listas e mapas | 107.152–128.186 | 100.229–117.441 |
| Alocação e ciclos | 13.644–15.668 | 12.281–17.514 |
| Matrizes | 5.175–7.040 | 5.308–6.351 |
| Recursão | 6.510–8.621 | 5.962–7.459 |
| Inicialização | 1.792–2.277 | 1.716–2.701 |

Dados: [ensaio com AST e as duas VMs](results/bytecode-step-02.json), [confirmação](results/bytecode-step-02-confirmation.json), [patch mantido](bytecode-steps/02-inline-dispatch.patch).

Comando do ensaio principal:

```sh
cargo build -p expressa --release --bin expressa --offline --locked
taskset -c 0 python3 benchmarks/run.py --engine ast=target/ast-reference-bd654b5/expressa-ast --engine vm00=target/bytecode-steps/00-baseline/expressa --engine vm02=target/bytecode-steps/02-inline-dispatch/expressa --backend ast=ast --backend vm00=bytecode --backend vm02=bytecode --repeat 31 --warmup 3 --seed 0 --output benchmarks/results/bytecode-step-02.json
```

A confirmação usa os mesmos dois binários VM, retira o motor AST, muda `--seed` para `1` e grava outro arquivo. Os nomes dos motores e seus hashes estão nos metadados; `working_source_sha256` corresponde ao candidato. O manifesto separado identifica as fontes de cada binário de referência. A próxima experiência deve usar `02-inline-dispatch` como referência, preservando a medição original e incluindo `00-baseline`/AST quando útil para acompanhar o resultado acumulado.

Validação da alteração mantida:

- 394 testes do núcleo passaram em debug e release (363 biblioteca, 21 regressões e 10 testes diferenciais/integração).
- `cargo clippy -p expressa --all-targets --offline --locked -- -D warnings` passou.
- `cargo fmt -p expressa --check` e `git diff --check` passaram.
- Hashes dos binários, fontes, arquivos de snapshot, patches e workloads foram conferidos contra os resultados; o binário release atual corresponde ao candidato medido.
- Os testes que escrevem temporários em `~/.cache` precisaram ser executados fora do sandbox, pois o diretório é somente leitura dentro dele.

Não houve alteração de dependências, sintaxe, semântica numérica, `avalie` ou backend AST. Em relação ao snapshot `00-baseline`, o único arquivo Rust alterado é `execute.rs`, com o atributo de inlining e seu comentário. Nenhum commit foi criado.


## Etapa 03: passagem de argumentos

A experiência seguinte é documentada separadamente em [BYTECODE-STEP-03.md](BYTECODE-STEP-03.md), com alterações, testes, medições e decisão. Sua referência é `02-inline-dispatch`.

## Etapa 04: parâmetros por posição

[Implementação, regressões, comparação CPU/RSS e validação](BYTECODE-STEP-04.md). Os testes de processos foram estabilizados em uma mudança separada antes desta medição.

## Etapa 05: instruções compactas

**Experimento descartado e implementação revertida após revisão do usuário.** [Contagem de armazenamento, comparação CPU/RSS e dados preservados](BYTECODE-STEP-05.md). A etapa 04 continua ativa.

## Próximas hipóteses

- Expandir a resolução por posições além dos parâmetros no escopo principal, com análise explícita de sombreamento, closures e avaliação dinâmica. Essa expansão ainda não está implementada.
- Definir separadamente o formato `.exb` e o carregamento, sem exigir a representação compacta descartada no runtime. Reavaliar compactação em memória somente com nova evidência de benefício.

Cache de `avalie` e firmware Pico continuam pendentes. O AST permanece disponível como referência.

## Etapa 06: formato EXB v1

Implementados compilação offline, carregamento validado e execução desktop de `.exb`, preservando a representação em memória anterior à etapa 05. [Contrato](../docs/exb.md), [tarefa](../lang-tasks/047-exb-file-format-and-loader.md) e [comparação antes/depois e fonte/EXB](EXB-V1-REVIEW.md). A serialização não implementa execução direta da flash nem reduz por si só a SRAM da VM.

## Etapa 07: perfil mínimo desktop

[Features e limites](../docs/runtime-profiles.md) e [comparação de tamanho/CPU/RSS](MINIMAL-PROFILE.md). O mesmo código gera um executável completo e outro sem executor AST, gráficos, processos e terminal interativo. O frontend e `avalie` são mantidos. A referência é a etapa 06 aceita, sem restauração da compactação da etapa 05.

## Etapa 08: referências e clone

[Contrato e limites](../docs/clone.md) e [comparação antes/depois de CPU, RSS, alocações e tamanho](CLONE.md). Implementação para revisão em AST, bytecode e EXB; inclui custos observados nos cenários gráficos. Esta etapa estabelece a semântica de cópia, sem prometer aceleração geral.

## Etapa 09: cópia interna de valores

[Implementação, perf e comparação de CPU/memória antes/depois](clone-fastpath/README.md). O percurso de pares foi separado para permitir melhor inlining das cópias comuns; a política dos recursos não clonáveis foi mantida. Ganhos e regressões estão registrados por caso.
