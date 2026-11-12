# Clone e referências — implementação e comparação antes/depois

Implementado para revisão, sem commits automáticos. [Contrato e limitações](../docs/clone.md), [tarefa 023](../lang-tasks/023-copy-and-alias-contract.md), [patch do runtime/testes](bytecode-steps/08-clone.patch). Esta é uma entrega de semântica e API; não promete acelerar programas existentes.

## Entrega

- `clone` copia grafos de listas/mapas/conjuntos/matrizes/pares, preservando aliases internos e ciclos. Falhas/cancelamento descartam shells parciais.
- Funções comprovadamente sem capturas recebem um ambiente separado, compartilhando código. Classificação conservadora, sobre instruções reais: consulta dinâmica e chamadas indiretas não comprovadas são recusadas. Não é um analisador completo de todas as funções possíveis.
- Mapas usam `:_clonavel` bool. Telas/documentos nativos têm uma marca interna que não desaparece com edição do campo ou cópia por `remova`. Módulos/processos impedem a cópia.
- Imagens passam a ter identidade compartilhada via `Rc<RefCell<Imagem>>`. Clone cria outro objeto com pixels inicialmente compartilhados; COW é possível no buffer. Snapshots continuam independentes. Não foram adicionados efeitos/edição de pixels na linguagem.
- Textos mantêm comportamento de valor e escrita parcial como substituição. O armazenamento continua `String`; não se apresenta compartilhamento físico de caracteres como implementado.
- Sem novos opcodes, mudança do layout de instruções ou revisão do formato EXB. Há mudança do tipo Rust de `Value::Imagem`; consumidores precisam adaptar construtores/matches conforme o embedding.

## Resultado e custos

| Executável release completo, não stripped | Bytes |
| --- | ---: |
| Antes | 4373720 |
| Candidato | 4488648 |

Aumento de 114928 bytes (112.23 KiB; 2.63%). Os dois usam o mesmo perfil/toolchain e features padrão. Não comparar diretamente com executáveis stripped de outros relatórios nem interpretar esse tamanho como firmware.

Nos programas comuns, os resultados de CPU são mistos e incluem regressões. Em particular, soma aritmética e closures regrediram em ambas as rodadas nos dois backends; os demais casos variam. Alterações de representação, código gerado e layout entram nessa comparação; não se atribui causalidade isolada sem perfilamento. Não se somam percentuais de etapas diferentes.

No cenário de sprites, há um custo consistente: VM com aproximadamente +3,0% e +6,7% de CPU nas duas rodadas. A sonda confirma 6.506 alocações adicionais no processo instrumentado, com 6.400 comandos de sprite; a identidade/snapshot de imagem agora envolve objetos com cabeçalho próprio. Os pixels não são duplicados por atribuição ou por cada clone de imagem. RSS do processo é ruidoso e não substitui a medida de buffers.

A sonda dos três workloads comuns registrou uma alocação adicional por execução capturada, coerente com o novo binding `clone` de cinco bytes; a quantidade de realocações permaneceu igual. Contadores de bytes vivos retornaram ao baseline depois da limpeza em todos os casos instrumentados. Isso descreve estes ensaios, não uma prova geral de ausência de vazamentos nem devolução de memória do malloc ao SO.

## Protocolo

[Manifesto e hashes](references/clone.json). Referência: fontes da árvore antes desta alteração, base Git `7c4745f2f3d1d5f6b9d03191626ededcb715b71b`, binário preservado em `08-before-clone`. Candidato: `08-clone`. Ambos são Expressa 0.5.0 completo e release/offline/locked, mesma configuração decimal/locale. Binários e `source.tar` locais ficam em `target/bytecode-steps/`; não houve commits novos do agente.

- Regressões comuns: nove `.lep`, quatro motores intercalados (`before_ast`, `before_vm`, `after_ast`, `after_vm`), 31 amostras e três aquecimentos por combinação, seeds 70/71. **2.232 amostras e 216 aquecimentos**.
- Gráficos: antes/depois com AST/bytecode, Conway completo 64×64 (25 gerações +3 de aquecimento) e 100 quadros com 64 sprites cada. Onze amostras e três aquecimentos por combinação, seeds 72/73. **176 amostras e 48 aquecimentos**.
- Todas as saídas/exit codes/stderr foram verificados. Conway usa referência independente de contagem/checksum; sprites compara o resultado final com uma imagem construída por retângulos. Nenhum caso abre janela ou cria viewer.
- Afinidade CPU 0. Nenhuma compilação ou suíte de testes executada durante as amostras temporais; edição leve de documentação e preparação de scripts continuou. Não houve controle completo de frequência, temperatura ou processos do SO.
- CPU usuário+sistema, parede monotônica e pico RSS do processo completo, incluindo leitura, parsing, compilação, execução e limpeza. Não são FPS nem percentis por frame. A API de apresentação da janela não foi modificada.

A sonda de alocações é separada, com outro executável e contador global que delega ao alocador System. Cada processo aquece uma execução, zera os contadores e mede outra chamada à API capturada. Leitura do fonte e inicialização inicial ficam fora; parsing/compilação, execução, verificação de saída e descarte de resultado ficam dentro. Há três processos por combinação, **54 amostras instrumentadas**. Os argumentos de arquivo são caminhos absolutos, iguais para antes/depois.

São contados pedidos de alloc/alloc_zeroed, realloc e dealloc; bytes pedidos por cada alocação/realocação; pico do delta de bytes vivos solicitados ao alocador e delta retido após a chamada. São bytes solicitados, sem metadados do malloc, fragmentação ou páginas residentes. `requested_bytes` soma o tamanho novo de cada realloc, não somente seu crescimento. Não comparar tempo dessas sondas com o CLI sem instrumentação.

As duas bibliotecas foram compiladas com release e a mesma instrumentação. A referência foi reconstruída de `git archive` do HEAD indicado, antes de suas bibliotecas serem copiadas/congeladas; o candidato usa os fontes novos. Bibliotecas/probes têm hashes distintos, registrados nos dados. Para reproduzir, usar diretórios de target separados entre checkouts para evitar colisão de artefatos Cargo e compilar a mesma sonda contra cada biblioteca. O caso `clone_graph` só existe no candidato: a referência não implementa `clone`, logo não há comparação antes/depois dessa operação.

## Regressões comuns — seed 70

CPU mediana em ms. Variações positivas significam mais CPU; valores negativos, menos.

| Caso | AST antes | AST depois | Δ CPU AST | VM antes | VM depois | Δ CPU VM |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| arguments | 127.242 | 127.624 | +0.30% | 64.520 | 66.119 | +2.48% |
| arithmetic | 65.433 | 67.329 | +2.90% | 51.285 | 53.144 | +3.62% |
| avalie | 4.857 | 4.761 | -1.98% | 6.329 | 6.435 | +1.67% |
| closures | 74.921 | 76.677 | +2.34% | 60.717 | 63.891 | +5.23% |
| collections | 66.523 | 66.613 | +0.14% | 55.867 | 56.341 | +0.85% |
| cycles | 8.664 | 8.622 | -0.48% | 7.156 | 7.423 | +3.73% |
| matrix | 3.162 | 3.124 | -1.20% | 3.182 | 3.166 | -0.50% |
| recursion | 9.598 | 9.649 | +0.53% | 3.097 | 3.193 | +3.10% |
| startup | 1.073 | 1.075 | +0.19% | 1.079 | 1.106 | +2.50% |

RSS mediano em KiB.

| Caso | AST antes | AST depois | Δ RSS AST | VM antes | VM depois | Δ RSS VM |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| arguments | 4736 | 4800 | +64 | 4788 | 4752 | -36 |
| arithmetic | 4724 | 4796 | +72 | 4724 | 4776 | +52 |
| avalie | 4792 | 4848 | +56 | 4788 | 4752 | -36 |
| closures | 4756 | 4788 | +32 | 4784 | 4788 | +4 |
| collections | 4860 | 4816 | -44 | 4824 | 4848 | +24 |
| cycles | 5072 | 5128 | +56 | 5080 | 5128 | +48 |
| matrix | 4988 | 5004 | +16 | 4952 | 5040 | +88 |
| recursion | 17100 | 17144 | +44 | 6300 | 6288 | -12 |
| startup | 4720 | 4784 | +64 | 4732 | 4784 | +52 |

[Amostras, tempos de parede e metadados](results/clone-70.json). Dispersão de CPU abaixo: P10–P90 em ms, interpolação linear em `(n−1)×p`. Não se afirma significância estatística.

| Caso | AST antes | AST depois | VM antes | VM depois |
| --- | ---: | ---: | ---: | ---: |
| arguments | 123.280–139.233 | 124.745–138.057 | 61.554–69.133 | 62.282–72.202 |
| arithmetic | 62.332–68.022 | 65.301–72.714 | 48.973–54.678 | 49.941–56.669 |
| avalie | 4.523–5.392 | 4.545–5.236 | 5.733–6.769 | 5.959–7.191 |
| closures | 72.818–83.375 | 73.078–81.283 | 58.859–68.380 | 60.125–69.300 |
| collections | 61.264–71.463 | 62.366–71.890 | 51.742–61.138 | 53.319–60.990 |
| cycles | 7.737–9.114 | 7.747–8.952 | 6.526–7.589 | 6.623–7.858 |
| matrix | 2.844–3.252 | 2.886–3.472 | 2.868–3.615 | 2.872–3.411 |
| recursion | 8.926–10.021 | 8.759–11.090 | 2.871–3.304 | 2.920–3.573 |
| startup | 0.998–1.130 | 1.030–1.170 | 0.992–1.164 | 1.047–1.172 |

## Regressões comuns — seed 71

CPU mediana em ms. Variações positivas significam mais CPU; valores negativos, menos.

| Caso | AST antes | AST depois | Δ CPU AST | VM antes | VM depois | Δ CPU VM |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| arguments | 126.126 | 127.982 | +1.47% | 65.292 | 65.897 | +0.93% |
| arithmetic | 65.197 | 67.213 | +3.09% | 52.048 | 52.597 | +1.05% |
| avalie | 5.043 | 5.070 | +0.54% | 6.356 | 6.373 | +0.27% |
| closures | 75.651 | 77.352 | +2.25% | 62.917 | 64.050 | +1.80% |
| collections | 65.533 | 66.198 | +1.01% | 55.180 | 55.996 | +1.48% |
| cycles | 8.848 | 8.860 | +0.14% | 7.452 | 7.306 | -1.96% |
| matrix | 3.152 | 3.186 | +1.08% | 3.199 | 3.188 | -0.34% |
| recursion | 9.677 | 9.576 | -1.04% | 3.212 | 3.178 | -1.06% |
| startup | 1.057 | 1.073 | +1.51% | 1.082 | 1.097 | +1.39% |

RSS mediano em KiB.

| Caso | AST antes | AST depois | Δ RSS AST | VM antes | VM depois | Δ RSS VM |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| arguments | 4712 | 4836 | +124 | 4780 | 4780 | +0 |
| arithmetic | 4704 | 4780 | +76 | 4724 | 4784 | +60 |
| avalie | 4792 | 4852 | +60 | 4776 | 4752 | -24 |
| closures | 4752 | 4780 | +28 | 4788 | 4788 | +0 |
| collections | 4820 | 4816 | -4 | 4840 | 4856 | +16 |
| cycles | 5064 | 5108 | +44 | 5112 | 5136 | +24 |
| matrix | 5000 | 4964 | -36 | 4980 | 5020 | +40 |
| recursion | 17088 | 17104 | +16 | 6284 | 6316 | +32 |
| startup | 4736 | 4788 | +52 | 4696 | 4752 | +56 |

[Amostras, tempos de parede e metadados](results/clone-71.json). Dispersão de CPU abaixo: P10–P90 em ms, interpolação linear em `(n−1)×p`. Não se afirma significância estatística.

| Caso | AST antes | AST depois | VM antes | VM depois |
| --- | ---: | ---: | ---: | ---: |
| arguments | 122.584–137.430 | 122.540–139.478 | 60.982–68.980 | 62.459–70.766 |
| arithmetic | 61.781–69.904 | 63.816–72.560 | 48.543–55.612 | 49.482–56.357 |
| avalie | 4.650–5.371 | 4.646–5.385 | 5.859–6.685 | 5.675–6.826 |
| closures | 71.675–80.663 | 73.820–82.243 | 58.621–66.967 | 60.821–68.614 |
| collections | 61.821–70.752 | 62.231–70.461 | 52.213–59.932 | 52.620–60.384 |
| cycles | 7.883–9.397 | 7.824–9.694 | 6.640–7.782 | 6.616–7.803 |
| matrix | 2.854–3.392 | 2.884–3.428 | 2.865–3.411 | 2.922–3.306 |
| recursion | 8.926–10.314 | 8.894–10.003 | 2.910–3.430 | 2.910–3.462 |
| startup | 0.990–1.316 | 1.022–1.231 | 0.980–1.147 | 1.011–1.181 |

## Gráficos raster — seed 72

CPU mediana em ms; RSS mediano em KiB. Tempos incluem o processo inteiro.

| Caso | Backend | CPU antes | CPU depois | Δ CPU | RSS antes | RSS depois | Δ RSS |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| sprites | ast | 16.793 | 17.284 | +2.92% | 5120 | 5128 | +8 |
| conway | ast | 572.635 | 584.342 | +2.04% | 5944 | 5908 | -36 |
| sprites | bytecode | 15.339 | 15.800 | +3.01% | 5152 | 5168 | +16 |
| conway | bytecode | 488.497 | 499.289 | +2.21% | 5928 | 5848 | -80 |

[Amostras individuais, parede e dispersão](results/clone-graphics-72.json). Não mede comunicação/renderização de janela nem latência de entrada.

## Gráficos raster — seed 73

CPU mediana em ms; RSS mediano em KiB. Tempos incluem o processo inteiro.

| Caso | Backend | CPU antes | CPU depois | Δ CPU | RSS antes | RSS depois | Δ RSS |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| sprites | ast | 17.063 | 17.405 | +2.00% | 5100 | 5120 | +20 |
| conway | ast | 581.059 | 584.203 | +0.54% | 5984 | 5884 | -100 |
| sprites | bytecode | 15.138 | 16.155 | +6.72% | 5088 | 5112 | +24 |
| conway | bytecode | 500.921 | 508.846 | +1.58% | 5888 | 5952 | +64 |

[Amostras individuais, parede e dispersão](results/clone-graphics-73.json). Não mede comunicação/renderização de janela nem latência de entrada.

## Alocações instrumentadas

Medianas de três processos. Bytes são pedidos ao alocador; não RSS. O pico é um delta em relação ao estado anterior à execução medida.

| Caso | Backend | Alloc antes | Alloc depois | Δ alloc | Δ bytes pedidos | Pico antes bytes | Pico depois bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| arithmetic | ast | 600341 | 600342 | +1 | +5 | 22728 | 22733 |
| arithmetic | bytecode | 364 | 365 | +1 | +5 | 25367 | 25372 |
| collections | ast | 700398 | 700399 | +1 | +13 | 25261 | 25274 |
| collections | bytecode | 400431 | 400432 | +1 | +13 | 30733 | 30738 |
| arguments | ast | 1650443 | 1650444 | +1 | +5 | 34382 | 34387 |
| arguments | bytecode | 550531 | 550532 | +1 | +5 | 40113 | 40118 |
| sprites | ast | 182231 | 188737 | +6506 | +365904 | 175864 | 187831 |
| sprites | bytecode | 147949 | 154455 | +6506 | +365904 | 159283 | 171250 |

O workload novo clona 200 vezes um grafo com ciclo, lista compartilhada e função sem capturas:

| Backend | Alloc | Realloc | Bytes pedidos | Pico delta bytes | Delta retido bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| ast | 21697 | 2024 | 1768264 | 226551 | 0 |
| bytecode | 16386 | 1844 | 1461680 | 225194 | 0 |

[Dados da sonda e hashes das bibliotecas](results/clone-allocations.json), [sonda Rust](clone_allocations.rs), [driver](clone_allocations.py), [casos](programs/clone/README.md). A maior contagem AST no caso novo inclui compilação temporária de funções para análise; bytecode reutiliza seus chunks. Não se mediu apenas o trecho interno de clone.

## Reprodução

Preservar os binários/fontes antes de substituir o runtime. Os diretórios abaixo são artefatos locais descritos no manifesto. Para novos ensaios, usar novos nomes de saída.

```sh
taskset -c 0 python3 benchmarks/run.py \
  --engine before_ast=target/bytecode-steps/08-before-clone/expressa \
  --engine before_vm=target/bytecode-steps/08-before-clone/expressa \
  --engine after_ast=target/bytecode-steps/08-clone/expressa \
  --engine after_vm=target/bytecode-steps/08-clone/expressa \
  --backend before_ast=ast --backend before_vm=bytecode \
  --backend after_ast=ast --backend after_vm=bytecode \
  --repeat 31 --warmup 3 --seed 70 --output benchmarks/results/clone-novo-70.json

# Repetir com seed 71 e outra saída. Gráficos:
taskset -c 0 python3 benchmarks/clone_graphics.py \
  --before target/bytecode-steps/08-before-clone/expressa \
  --after target/bytecode-steps/08-clone/expressa \
  --helper target/benchmarks/conway-baseline-c2de421/measure \
  --repeat 11 --warmup 3 --seed 72 --output benchmarks/results/clone-graphics-novo-72.json

# Sonda: compilar a mesma fonte contra cada biblioteca, em targets separados.
rustc --edition=2024 -O benchmarks/clone_allocations.rs \
  --extern expressa=CAMINHO/libexpressa.rlib -L dependency=CAMINHO/deps -o CAMINHO/probe
python3 benchmarks/clone_allocations.py \
  --before target/benchmarks/clone-allocations/before \
  --after target/benchmarks/clone-allocations/after \
  --manifest target/benchmarks/clone-allocations/manifest.json \
  --repeat 3 --output benchmarks/results/clone-allocations-novo.json
```

## Validação e limites

- Workspace completo: 647 testes das suítes principais passaram; quatro helpers intencionalmente ignorados, alguns invocados em subprocessos.
- Núcleo completo em release: 444 testes passaram; um helper ignorado.
- Perfil mínimo: 333 testes passaram, incluindo dez testes de clone aplicáveis ao perfil.
- Clippy de todos os targets no workspace e no perfil mínimo com `-D warnings`, formatação e `git diff --check` aprovados.
- Testes verificam ciclos/aliases, isolamento, recursos, módulos/processos, textos, funções, controle de fluxo, cancelamento com semântica de saída preservada, imagens/snapshots e COW por host. Há testes contra nomes de chamada forjados e texto de depuração não executável.
- Contagens de amostras/medianas, hashes de fontes, binários, arquivos, bibliotecas, probes e runners conferidos. Patch aplicado numa cópia temporária da referência e comparado aos fontes/testes atuais.

A classificação de funções é conservadora, não completa; callbacks indiretos e `ordene` podem ser recusados mesmo em casos seguros. Há limites de trabalho/depth, sem orçamento global de heap. Não há mudança de semântica numérica, sintaxe de referências, teclado ou API de apresentação de frames, firmware ou medição em Raspberry Pi. No_std continua fora desta etapa. A coleta de desempenho desta entrega pode ser revisada junto com os custos antes de qualquer commit manual.
