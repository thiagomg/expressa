# Etapa 09: caminho comum de Value::clone

Implementação para revisão, sem commits. Referência: entrega de clone e referências já aceita em `f709c0a`, binário congelado `08-clone`. Candidato: `09-clone-fastpath`. [Investigação anterior](../clone-investigation/README.md).

## Mudança

Apenas `src/runtime/value.rs` muda no runtime. O corpo de `Value::clone` mantém cópias de escalares e referências pequenas e desloca a cópia de pares para `clone_pair`, com `#[inline(never)]`. O percurso iterativo, o caso de par simples e o compartilhamento das coleções continuam iguais. A separação permite inserir o caminho comum nos chamadores sem carregar a travessia, alocação e unwind do percurso de pares.

Esta é uma otimização da cópia interna de valores, não uma nova implementação da nativa `clone`. A semântica aprovada, análise de funções, COW de pixels, textos e EXB permanecem iguais. A política de recursos não clonáveis (`:_clonavel` e marca interna) é mantida, incluindo seu custo de armazenamento nos mapas; não foi removida nem renomeada.

Um teste de regressão copia 50.000 pares em uma thread com 128 KiB de pilha, confere cada chave e a identidade compartilhada da lista no final, e desmonta as duas cadeias iterativamente. O objetivo é impedir que uma simplificação para Clone recursivo volte a introduzir estouro de pilha.

## Resultado

Na VM, houve redução de CPU nas duas rodadas em argumentos (1,3–4,0%), aritmética (3,1%), closures (3,3–4,3%), coleções (0,5–4,2%), sprites (0,9–3,2%) e Conway raster (0,6–3,2%). Há regressões: o teste curto de alocação e ciclos usou 1,5–4,1% mais CPU na VM e 0,9–5,6% mais no AST. Não se atribui essa regressão isoladamente a um mecanismo sem outro experimento. O ganho não é universal.

Os contadores de `perf` corroboram menos trabalho na VM: na aritmética, instruções −0,90% e ciclos −2,21%; em closures, instruções −1,35% e ciclos −2,12%. São programas cinco vezes maiores que os casos curtos, em rodada separada. Menos instruções não implica a mesma porcentagem de redução de CPU, e percentuais de etapas diferentes não se somam.

Todos os contadores da sonda de alocações foram idênticos antes/depois nos seis casos, nos dois backends e nas três repetições. Os picos de bytes vivos solicitados ao alocador também foram iguais. RSS do processo variou e é registrado abaixo; não se apresenta isso como redução garantida do heap.

| Executável release completo, não stripped | Bytes |
| --- | ---: |
| Antes (`08-clone`) | 4488648 |
| Depois (`09-clone-fastpath`) | 4474528 |

Redução de 14.120 bytes (13,79 KiB; 0,31%). Mesmo perfil/toolchain/features. Este tamanho não é firmware nem SRAM de uma placa.

## Protocolo

[Manifesto e hashes](manifest.json), [patch de runtime/teste](../bytecode-steps/09-clone-fastpath.patch). Binários, bibliotecas e arquivos-fonte congelados ficam em `target/bytecode-steps` e `target/clone-fastpath`, sem commits novos. As execuções temporais ocorreram depois de todas as compilações/testes e sem concorrência com sondas de alocação. Consultas leves a arquivos e preparação da documentação continuaram.

- Mesmo host/toolchain da investigação anterior: i5-10210U, Linux 7.3.0, Rust 1.98.1; release padrão, features completas, decimal/en, afinidade CPU 0.
- Casos comuns: nove programas, quatro motores antes/depois × AST/VM, 31 amostras +3 aquecimentos, seeds 85/86: **2.232 amostras e 216 aquecimentos**.
- Gráficos: sprites e Conway completo 64×64, 25 gerações +3 aquecimentos internos, rasterização sem viewer; antes/depois × AST/VM, 11 amostras +3 aquecimentos, seeds 87/88: **176 amostras e 48 aquecimentos**.
- Hardware: antes/depois × AST/VM, aritmética de um milhão de iterações e closures de 500 mil; 11 amostras +2 aquecimentos, seed 89: **88 amostras e 16 aquecimentos**.
- `perf` 7.3.0, `perf_event_paranoid=2`, somente o processo e espaço de usuário: `cycles:u`, `instructions:u`, `branches:u`, `branch-misses:u`. Cobertura dos contadores conferida, sem multiplexação inferior a 99%. Não foram necessárias novas alterações de configuração do sistema.
- Quatro perfis de ciclos separados, somente VM: antes/depois em aritmética de dez milhões de iterações e closures de cinco milhões, frequência solicitada 997 Hz. Relatórios usam símbolos folha, sem atribuir linhas ou supor call chains DWARF confiáveis no binário otimizado. Não houve amostras perdidas nesses perfis.
- Sonda com seis casos: aritmética, coleções, argumentos, alocação/ciclos, grafo com clone e sprites; antes/depois × AST/VM ×3 = **72 amostras**. Aquece a API capturada antes de contar; inclui parsing/compilação, execução e limpeza. Bytes são pedidos ao alocador, não RSS nem tamanho útil de blocos do malloc.
- Saídas, exit codes e stderr dos programas foram conferidos em cada execução. Gráficos têm referência independente. Os tempos incluem leitura, parsing/compilação, execução e limpeza; não são medidas de FPS ou somente do loop da VM.
- Não houve controle completo de frequência, temperatura, ASLR ou outros processos do SO. As tabelas não afirmam significância estatística. Consultar amostras e dispersão, especialmente em programas de poucos milissegundos.

## Casos comuns — seed 85

CPU mediana em ms; positivo significa mais CPU.

| Caso | AST antes | AST depois | Δ AST | VM antes | VM depois | Δ VM |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| arguments | 128.855 | 123.992 | -3.77% | 65.326 | 62.738 | -3.96% |
| arithmetic | 66.466 | 66.307 | -0.24% | 52.128 | 50.526 | -3.07% |
| avalie | 4.776 | 4.929 | +3.20% | 6.326 | 6.285 | -0.65% |
| closures | 76.185 | 75.046 | -1.50% | 62.616 | 60.579 | -3.25% |
| collections | 64.904 | 65.206 | +0.47% | 55.511 | 53.176 | -4.21% |
| cycles | 8.772 | 8.847 | +0.85% | 7.411 | 7.522 | +1.50% |
| matrix | 3.013 | 2.988 | -0.83% | 3.142 | 3.056 | -2.74% |
| recursion | 9.510 | 9.703 | +2.03% | 3.159 | 3.160 | +0.03% |
| startup | 1.035 | 1.055 | +1.93% | 1.059 | 1.103 | +4.15% |

RSS mediano em KiB, processo inteiro.

| Caso | AST antes | AST depois | Δ AST KiB | VM antes | VM depois | Δ VM KiB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| arguments | 4720 | 4556 | -164 | 4692 | 4624 | -68 |
| arithmetic | 4628 | 4564 | -64 | 4692 | 4592 | -100 |
| avalie | 4712 | 4612 | -100 | 4676 | 4624 | -52 |
| closures | 4676 | 4604 | -72 | 4684 | 4592 | -92 |
| collections | 4752 | 4624 | -128 | 4752 | 4688 | -64 |
| cycles | 5004 | 4892 | -112 | 5044 | 4928 | -116 |
| matrix | 4848 | 4764 | -84 | 4936 | 4772 | -164 |
| recursion | 17040 | 16900 | -140 | 6208 | 6128 | -80 |
| startup | 4656 | 4520 | -136 | 4700 | 4584 | -116 |

[Amostras, parede, CPU e metadados](../results/clone-fastpath-85.json).

## Casos comuns — seed 86

CPU mediana em ms; positivo significa mais CPU.

| Caso | AST antes | AST depois | Δ AST | VM antes | VM depois | Δ VM |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| arguments | 124.768 | 121.117 | -2.93% | 62.477 | 61.681 | -1.27% |
| arithmetic | 65.575 | 64.237 | -2.04% | 50.472 | 48.909 | -3.10% |
| avalie | 4.777 | 4.695 | -1.72% | 5.754 | 5.828 | +1.29% |
| closures | 73.372 | 71.849 | -2.08% | 62.799 | 60.092 | -4.31% |
| collections | 62.645 | 62.382 | -0.42% | 53.267 | 52.973 | -0.55% |
| cycles | 8.097 | 8.553 | +5.63% | 6.942 | 7.225 | +4.08% |
| matrix | 2.936 | 2.984 | +1.63% | 2.946 | 2.982 | +1.22% |
| recursion | 9.334 | 9.170 | -1.76% | 3.110 | 3.018 | -2.96% |
| startup | 1.052 | 1.052 | +0.00% | 1.077 | 1.078 | +0.09% |

RSS mediano em KiB, processo inteiro.

| Caso | AST antes | AST depois | Δ AST KiB | VM antes | VM depois | Δ VM KiB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| arguments | 4792 | 4672 | -120 | 4772 | 4780 | +8 |
| arithmetic | 4768 | 4680 | -88 | 4780 | 4752 | -28 |
| avalie | 4844 | 4716 | -128 | 4788 | 4792 | +4 |
| closures | 4772 | 4756 | -16 | 4780 | 4780 | +0 |
| collections | 4824 | 4796 | -28 | 4860 | 4848 | -12 |
| cycles | 5080 | 5044 | -36 | 5088 | 5132 | +44 |
| matrix | 4976 | 4920 | -56 | 4936 | 4968 | +32 |
| recursion | 17108 | 17096 | -12 | 6288 | 6296 | +8 |
| startup | 4756 | 4672 | -84 | 4772 | 4776 | +4 |

[Amostras, parede, CPU e metadados](../results/clone-fastpath-86.json).

## Gráficos sem janela

CPU mediana em ms; RSS em KiB.

| Seed | Caso | Backend | CPU antes | CPU depois | Δ CPU | RSS antes | RSS depois |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| 87 | sprites | ast | 17.149 | 17.316 | +0.97% | 5144 | 5068 |
| 87 | conway | ast | 582.839 | 578.496 | -0.75% | 5856 | 5908 |
| 87 | sprites | bytecode | 16.228 | 15.713 | -3.17% | 5176 | 5216 |
| 87 | conway | bytecode | 498.915 | 482.928 | -3.20% | 5888 | 6008 |
| 88 | sprites | ast | 17.114 | 17.046 | -0.40% | 5264 | 5148 |
| 88 | conway | ast | 583.406 | 577.985 | -0.93% | 5972 | 5956 |
| 88 | sprites | bytecode | 15.805 | 15.656 | -0.94% | 5300 | 5180 |
| 88 | conway | bytecode | 497.429 | 494.604 | -0.57% | 5932 | 5980 |

Amostras: [87](../results/clone-fastpath-graphics-87.json), [88](../results/clone-fastpath-graphics-88.json).

## Contadores de hardware

Medianas absolutas por evento. Os contadores excluem o kernel; a CPU usuário+sistema dos ensaios anteriores inclui trabalho no kernel. As medianas de eventos diferentes não necessariamente pertencem ao mesmo processo.

| Backend | Caso | Instruções antes | Instruções depois | Δ instruções | Ciclos antes | Ciclos depois | Δ ciclos |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| ast | arithmetic | 3383482918 | 3355483675 | -0.83% | 1204089989 | 1205227753 | +0.09% |
| ast | closures | 3792693482 | 3751692762 | -1.08% | 1379089744 | 1362168911 | -1.23% |
| bytecode | arithmetic | 2876517153 | 2850518955 | -0.90% | 934841944 | 914229149 | -2.20% |
| bytecode | closures | 3186591652 | 3143592416 | -1.35% | 1104127543 | 1080677478 | -2.12% |

Branches e erros de predição estão nos [dados brutos de perf](../results/clone-fastpath-perf-89.json), junto a cobertura e runtime de cada evento. Não se generaliza a redução de branch misses de um caso para todo o interpretador.

## Perfis por símbolo

Nos perfis finais e separados, a participação amostrada da função out-of-line `Value::clone` na VM foi:

| Caso | Antes | Depois |
| --- | ---: | ---: |
| arithmetic | 1.98% | Não amostrado |
| closures | 1.82% | 0.25% |

Isso não significa que a cópia tenha custo zero: o código inserido nos chamadores passa a ser atribuído aos símbolos deles. Os relatórios completos constam dos dados de perf; o `.data` local é preservado com hash. Perfis exploratórios anteriores com captura de pilha não são o denominador destas tabelas. Busca de bindings e hashing permanecem entre os custos principais; ampliar a resolução por posição é outra mudança, ainda não implementada nesta etapa.

## Alocações

Todos os campos medidos foram iguais entre as versões em cada combinação. A tabela mostra as medianas de ambas, não uma soma das duas versões.

| Caso | Backend | Alloc | Realloc | Bytes pedidos | Pico delta de bytes vivos | Delta retido |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| arithmetic | ast | 600342 | 5 | 1839253 | 22733 | 0 |
| arithmetic | bytecode | 365 | 11 | 44545 | 25372 | 0 |
| collections | ast | 700399 | 6 | 3545810 | 25274 | 0 |
| collections | bytecode | 400432 | 18 | 1158319 | 30738 | 0 |
| arguments | ast | 1650444 | 10 | 124112905 | 34387 | 0 |
| arguments | bytecode | 550532 | 34 | 37882756 | 40118 | 0 |
| cycles | ast | 105469 | 242 | 5088316 | 227143 | 0 |
| cycles | bytecode | 75502 | 249 | 4985170 | 224692 | 0 |
| clone_graph | ast | 21697 | 2024 | 1768264 | 226551 | 0 |
| clone_graph | bytecode | 16386 | 1844 | 1461680 | 225194 | 0 |
| sprites | ast | 188737 | 6844 | 10739223 | 187831 | 0 |
| sprites | bytecode | 154455 | 6863 | 10547157 | 171250 | 0 |

[Dados das 72 amostras](../results/clone-fastpath-allocations.json). Delta retido zero nesses casos não é prova geral de ausência de vazamentos. O cenário de alocação/ciclos regrediu em CPU apesar de manter os mesmos pedidos de memória.

## Validação

- Workspace completo: **648 testes principais passaram**, quatro helpers ignorados intencionalmente; três helpers também foram executados em subprocessos.
- Perfil mínimo (`--no-default-features`): **334 testes passaram**.
- Release: teste da cadeia de 50.000 pares e os **13 testes de clone** passaram; estes cobrem AST, bytecode e EXB quando disponíveis.
- Clippy do workspace e do perfil mínimo, todos os targets, com `-D warnings`, formatação e `git diff --check` aprovados.
- Dois testes existentes do perfil mínimo precisaram de nova execução fora do sandbox para gravar em `~/.cache`; a execução final passou. A suíte completa já foi executada com esse acesso.
- Hashes de fontes/binários/arquivos/bibliotecas/sondas/runners, contagens e medianas conferidos. Patch aplicado a uma cópia temporária da referência e comparado ao fonte candidato.

## Reprodução

Preservar os artefatos de referência da etapa 08. O candidato é compilado com `cargo build -p expressa --lib --bin expressa --release --offline --locked`. Não executar builds/testes/sondas ao mesmo tempo que medições temporais. Os diretórios e hashes dos artefatos locais estão no manifesto. Novos ensaios devem usar outras saídas.

```sh
taskset -c 0 python3 benchmarks/run.py \
  --engine before_ast=target/bytecode-steps/08-clone/expressa \
  --engine before_vm=target/bytecode-steps/08-clone/expressa \
  --engine after_ast=target/bytecode-steps/09-clone-fastpath/expressa \
  --engine after_vm=target/bytecode-steps/09-clone-fastpath/expressa \
  --backend before_ast=ast --backend before_vm=bytecode \
  --backend after_ast=ast --backend after_vm=bytecode \
  --repeat 31 --warmup 3 --seed 85 --output benchmarks/results/clone-fastpath-novo-85.json
# Repetir com seed 86.

taskset -c 0 python3 benchmarks/clone_graphics.py \
  --before target/bytecode-steps/08-clone/expressa \
  --after target/bytecode-steps/09-clone-fastpath/expressa \
  --helper target/benchmarks/conway-baseline-c2de421/measure \
  --repeat 11 --warmup 3 --seed 87 --output benchmarks/results/clone-fastpath-graphics-novo-87.json
# Repetir com seed 88.

taskset -c 0 python3 benchmarks/clone-fastpath/perf.py \
  --output benchmarks/results/clone-fastpath-perf-novo.json

# Compilar a mesma sonda contra cada biblioteca congelada e seus deps compatíveis.
rustc --edition=2024 -O benchmarks/clone_allocations.rs \
  --extern expressa=CAMINHO/libexpressa.rlib -L dependency=CAMINHO/deps -o CAMINHO/probe
python3 benchmarks/clone-fastpath/allocations.py \
  --manifest target/clone-fastpath/allocations/manifest.json \
  --output benchmarks/results/clone-fastpath-allocations-novo.json
```

Os scripts de perf e alocações registram fontes, executáveis e opções. O manifesto da sonda deve corresponder às bibliotecas/probes escolhidos. Use targets separados ao reconstruir outra referência, evitando colisões de artefatos Cargo. Não há medição em Raspberry Pi ou alteração de no_std nesta entrega.
