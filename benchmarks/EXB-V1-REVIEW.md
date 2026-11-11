# EXB v1 — revisão desktop e medições

Implementado para revisão, sem commits automáticos. Esta etapa entrega um formato persistente e carregamento validado; não é uma otimização do loop da VM. A compactação da etapa 05 continua revertida. [Contrato e CLI](../docs/exb.md), [tarefa 047](../lang-tasks/047-exb-file-format-and-loader.md), [patch completo de src/tests](bytecode-steps/06-exb-v1.patch).

## Conclusão

Nos nove programas, EXB não apresentou redução consistente de CPU em relação ao mesmo executável lendo fonte. As diferenças entre medianas vão de −2,4% a +5,2% na rodada 40 e de −0,9% a +3,2% na rodada 41. Os sinais mudam entre rodadas em vários casos. Os programas são pequenos: a leitura e verificação podem custar tanto quanto a análise/compilação evitadas. Não extrapolar para programas maiores nem interpretar esses números como execução isolada da VM.

O caminho `.lep` novo teve CPU menor na maioria dos casos, mas não houve otimização deliberada da execução nesta etapa. Mudanças de código/layout do executável e variação de medição podem explicar isso; não se atribui causalidade ao formato EXB. Matriz regrediu em ambas as rodadas.

Há custo observável no executável e no RSS: binário de 4.229.824 para 4.358.720 bytes (+128.896 bytes; +3,05%). Para `.lep`, RSS mediano aumentou 292–428 KiB na rodada 40 e 156–280 KiB na rodada 41. EXB versus fonte no novo binário variou de −148 a +88 KiB e de −172 a +36 KiB, respectivamente. RSS é uma medida do processo inteiro, não do heap nem da SRAM do Pico.

## Protocolo e referências

Duas rodadas intercaladas, seeds 40 e 41, nove casos, três caminhos, 51 amostras por combinação e cinco aquecimentos. Total: 2.754 amostras medidas e 270 aquecimentos. Afinidade na CPU 0, mesma máquina/toolchain/configuração decimal, builds release offline/locked. Cada execução deve terminar com sucesso e imprimir exatamente `verdadeiro`. Nenhuma compilação ou suíte de testes foi executada durante as amostras; documentação leve foi editada nesse intervalo.

- `before`: binário anterior à funcionalidade, lendo `.lep` (a compactação já estava revertida).
- `source`: novo binário lendo `.lep`, incluindo parsing e compilação.
- `compiled`: mesmo novo binário lendo `.exb`, incluindo leitura, decodificação e verificação. Geração offline excluída.

Todos usam backend bytecode. A comparação com AST permanece no [relatório anterior](AST-VS-BYTECODE.md); esta experiência isola a funcionalidade EXB. Os tempos são do processo completo, incluindo inicialização e encerramento. A razão CPU/tempo de parede não é uma medida de eficiência.

[Manifesto, hashes e referências](references/exb-v1.json). Os binários e arquivos fonte congelados estão nos diretórios locais indicados pelo manifesto; o patch reproduz a mudança sobre o arquivo de referência e foi aplicado em diretório temporário para conferir todos os fontes/testes. A fixture binária fixa no teste também protege compatibilidade do formato v1 sem depender da estratégia atual do compilador.

```sh
cargo build -p expressa --release --bin expressa --offline --locked
mkdir -p target/benchmarks/exb-v1
for source in benchmarks/programs/*.lep; do
  name=$(basename "$source" .lep)
  target/bytecode-steps/06-exb-v1/expressa compilar "$(realpath "$source")" -o "target/benchmarks/exb-v1/$name.exb"
done
# Repetir com seed 41 e outro arquivo de saída:
taskset -c 0 python3 benchmarks/run.py \
  --engine before=target/bytecode-steps/05-before-compact-instructions/expressa \
  --engine source=target/bytecode-steps/06-exb-v1/expressa \
  --engine compiled=target/bytecode-steps/06-exb-v1/expressa \
  --backend before=bytecode --backend source=bytecode --backend compiled=bytecode \
  --exb compiled=target/benchmarks/exb-v1 \
  --repeat 51 --warmup 5 --seed 40 --output benchmarks/results/exb-v1-40.json
```

Preservar/reconstruir os binários congelados antes do comando; o build não os cria automaticamente. Os nomes de fonte embutidos são caminhos absolutos nesta medição, registrados nos hashes dos artefatos. Compilar com outro nome muda o arquivo EXB.

## Comparações por rodada

Variações positivas significam mais CPU/RSS; negativas, menos. Não se calcula média de ganhos entre programas.

### Rodada 40

| Caso | CPU fonte/antes | CPU EXB/fonte | RSS fonte−antes KiB | RSS EXB−fonte KiB |
| --- | ---: | ---: | ---: | ---: |
| arguments | -1.34% | -0.01% | +292 | +88 |
| arithmetic | -2.39% | +1.15% | +404 | +44 |
| avalie | -2.46% | -2.37% | +356 | +68 |
| closures | -2.86% | -0.12% | +348 | +40 |
| collections | -0.43% | -1.78% | +368 | -4 |
| cycles | -6.28% | +0.34% | +324 | +28 |
| matrix | +0.86% | +5.23% | +372 | -16 |
| recursion | -6.01% | -1.38% | +396 | -148 |
| startup | -7.45% | -1.31% | +428 | +36 |

### Medianas e dispersão — 40

[Dados brutos](results/exb-v1-40.json). Cada célula mostra mediana [P10–P90]; percentis por interpolação linear em `(n−1)×p`. CPU = usuário + sistema por amostra; medianas das partes não precisam somar à mediana do total. As amostras também preservam CPU usuário/sistema e utilização.

| Caso | Caminho | CPU ms | Parede ms | Pico RSS KiB |
| --- | --- | ---: | ---: | ---: |
| arguments | before | 64.787 [62.074–68.722] | 65.189 [62.455–69.388] | 4664 [4552–4748] |
| arguments | source | 63.922 [60.549–69.199] | 64.368 [60.876–69.796] | 4956 [4852–5072] |
| arguments | compiled | 63.915 [60.515–68.432] | 64.583 [60.861–69.143] | 5044 [4912–5112] |
| arithmetic | before | 52.073 [49.304–55.918] | 52.526 [49.684–56.505] | 4580 [4484–4728] |
| arithmetic | source | 50.828 [48.391–55.941] | 51.292 [48.749–56.547] | 4984 [4800–5060] |
| arithmetic | compiled | 51.414 [49.250–55.763] | 51.741 [49.581–56.241] | 5028 [4960–5112] |
| avalie | before | 6.263 [5.576–7.193] | 6.437 [5.721–7.376] | 4620 [4488–4720] |
| avalie | source | 6.109 [5.531–6.694] | 6.270 [5.668–6.925] | 4976 [4876–5080] |
| avalie | compiled | 5.964 [5.563–6.643] | 6.095 [5.701–6.866] | 5044 [4920–5112] |
| closures | before | 62.785 [59.543–67.123] | 63.201 [59.951–67.700] | 4636 [4516–4740] |
| closures | source | 60.988 [58.111–66.746] | 61.338 [58.482–67.433] | 4984 [4848–5072] |
| closures | compiled | 60.913 [58.282–65.408] | 61.422 [58.720–66.169] | 5024 [4944–5112] |
| collections | before | 55.042 [52.219–58.788] | 55.573 [52.597–59.303] | 4728 [4556–4856] |
| collections | source | 54.808 [51.012–57.630] | 55.288 [51.369–58.292] | 5096 [4976–5192] |
| collections | compiled | 53.833 [51.530–58.652] | 54.220 [51.823–59.124] | 5092 [4984–5200] |
| cycles | before | 7.458 [6.641–8.087] | 7.618 [6.780–8.285] | 5004 [4812–5100] |
| cycles | source | 6.990 [6.374–7.568] | 7.156 [6.516–7.906] | 5328 [5180–5432] |
| cycles | compiled | 7.014 [6.324–7.882] | 7.202 [6.473–8.140] | 5356 [5268–5456] |
| matrix | before | 2.899 [2.796–3.496] | 3.032 [2.914–3.764] | 4836 [4764–5016] |
| matrix | source | 2.924 [2.775–3.182] | 3.075 [2.892–3.320] | 5208 [5048–5304] |
| matrix | compiled | 3.077 [2.773–3.414] | 3.208 [2.896–3.592] | 5192 [5096–5272] |
| recursion | before | 3.078 [2.894–3.537] | 3.218 [3.032–3.733] | 6180 [6076–6284] |
| recursion | source | 2.893 [2.763–3.469] | 3.027 [2.897–3.731] | 6576 [6432–6700] |
| recursion | compiled | 2.853 [2.740–3.288] | 2.985 [2.883–3.492] | 6428 [6304–6504] |
| startup | before | 1.074 [1.013–1.338] | 1.188 [1.120–1.529] | 4580 [4484–4720] |
| startup | source | 0.994 [0.927–1.128] | 1.115 [1.023–1.290] | 5008 [4808–5072] |
| startup | compiled | 0.981 [0.923–1.113] | 1.101 [1.028–1.272] | 5044 [4920–5108] |

### Rodada 41

| Caso | CPU fonte/antes | CPU EXB/fonte | RSS fonte−antes KiB | RSS EXB−fonte KiB |
| --- | ---: | ---: | ---: | ---: |
| arguments | -5.64% | +0.33% | +200 | +8 |
| arithmetic | -0.93% | +3.03% | +280 | +12 |
| avalie | -1.88% | +2.57% | +196 | +36 |
| closures | -2.92% | +2.17% | +240 | -20 |
| collections | -4.23% | +1.12% | +224 | +0 |
| cycles | -6.93% | +1.52% | +176 | -32 |
| matrix | +2.08% | -0.86% | +232 | -4 |
| recursion | -5.15% | +3.17% | +156 | -172 |
| startup | -5.35% | +0.97% | +224 | -8 |

### Medianas e dispersão — 41

[Dados brutos](results/exb-v1-41.json). Cada célula mostra mediana [P10–P90]; percentis por interpolação linear em `(n−1)×p`. CPU = usuário + sistema por amostra; medianas das partes não precisam somar à mediana do total. As amostras também preservam CPU usuário/sistema e utilização.

| Caso | Caminho | CPU ms | Parede ms | Pico RSS KiB |
| --- | --- | ---: | ---: | ---: |
| arguments | before | 66.721 [62.962–71.040] | 67.268 [63.348–71.848] | 4616 [4488–4752] |
| arguments | source | 62.961 [60.196–69.093] | 63.379 [60.605–69.746] | 4816 [4736–4896] |
| arguments | compiled | 63.169 [60.736–68.600] | 63.645 [61.068–69.188] | 4824 [4712–4972] |
| arithmetic | before | 50.785 [48.587–56.388] | 51.172 [48.912–57.028] | 4556 [4488–4684] |
| arithmetic | source | 50.312 [48.286–56.224] | 50.665 [48.644–56.816] | 4836 [4716–4912] |
| arithmetic | compiled | 51.835 [48.615–57.620] | 52.315 [48.920–58.264] | 4848 [4700–4952] |
| avalie | before | 6.263 [5.612–7.350] | 6.453 [5.742–7.636] | 4612 [4480–4708] |
| avalie | source | 6.145 [5.530–6.767] | 6.328 [5.650–7.016] | 4808 [4764–4896] |
| avalie | compiled | 6.303 [5.613–7.544] | 6.444 [5.747–7.857] | 4844 [4700–4996] |
| closures | before | 62.247 [59.725–68.862] | 62.718 [60.086–69.636] | 4612 [4492–4700] |
| closures | source | 60.428 [58.520–65.820] | 60.814 [58.847–66.335] | 4852 [4776–4904] |
| closures | compiled | 61.740 [58.573–67.082] | 62.130 [59.039–67.793] | 4832 [4672–4936] |
| collections | before | 56.474 [52.488–61.618] | 56.852 [52.925–62.797] | 4684 [4592–4776] |
| collections | source | 54.084 [50.742–60.273] | 54.475 [51.085–60.975] | 4908 [4832–5004] |
| collections | compiled | 54.689 [51.006–60.322] | 55.187 [51.393–61.148] | 4908 [4772–4996] |
| cycles | before | 7.432 [6.675–7.864] | 7.656 [6.840–8.100] | 4992 [4868–5112] |
| cycles | source | 6.917 [6.391–7.467] | 7.109 [6.522–7.681] | 5168 [5084–5264] |
| cycles | compiled | 7.022 [6.403–7.480] | 7.192 [6.547–7.674] | 5136 [5036–5228] |
| matrix | before | 3.072 [2.764–3.360] | 3.204 [2.878–3.517] | 4812 [4720–5000] |
| matrix | source | 3.136 [2.782–3.345] | 3.269 [2.904–3.550] | 5044 [4984–5112] |
| matrix | compiled | 3.109 [2.781–3.450] | 3.237 [2.902–3.650] | 5040 [4908–5116] |
| recursion | before | 3.164 [2.868–3.616] | 3.316 [2.999–3.854] | 6200 [6076–6284] |
| recursion | source | 3.001 [2.847–3.724] | 3.137 [2.985–3.967] | 6356 [6292–6452] |
| recursion | compiled | 3.096 [2.874–3.302] | 3.231 [2.998–3.523] | 6184 [6032–6328] |
| startup | before | 1.085 [0.999–1.211] | 1.207 [1.118–1.404] | 4612 [4488–4724] |
| startup | source | 1.027 [0.971–1.224] | 1.143 [1.078–1.353] | 4836 [4768–4896] |
| startup | compiled | 1.037 [0.971–1.230] | 1.154 [1.080–1.415] | 4828 [4592–4960] |

## Tamanho dos programas

O formato inclui fonte completa, nomes e posições para diagnóstico. Estes artefatos são maiores que os fontes curtos e não representam compressão. O nome absoluto do arquivo também entra na conta.

| Caso | Fonte bytes | EXB bytes |
| --- | ---: | ---: |
| arguments | 219 | 2230 |
| arithmetic | 75 | 791 |
| avalie | 141 | 1392 |
| closures | 134 | 1423 |
| collections | 128 | 1514 |
| cycles | 97 | 981 |
| matrix | 78 | 1052 |
| recursion | 86 | 1090 |
| startup | 20 | 418 |

## Limitações e próximos passos

EXB continua decodificado para o mesmo enum em memória (16 bytes por instrução). Não há execução direta da flash, remoção do frontend, bundle, firmware, heap global limitado nem medição em hardware Pico. `avalie` e imports `.lep` ainda precisam do frontend. O checksum não autentica os arquivos. Limites de decodificação/verificação são um perfil desktop, não garantia de consumo total.

Uma próxima medição voltada ao carregamento deve usar fontes maiores e separar compilação, leitura/verificação e execução repetida de `CompiledProgram` em estado novo. Isso permitiria identificar onde o trabalho foi economizado ou adicionado sem confundir código de execução com inicialização.

## Validação funcional final

- `cargo test --workspace --offline --locked`: 622 testes das suítes principais passaram; quatro helpers intencionalmente ignorados, alguns invocados separadamente pelos testes de subprocessos.
- `cargo test -p expressa --release --lib --test ast_regressions --test bytecode_conformance --test exb --offline --locked`: 419 passaram; um helper ignorado.
- `EXPRESSA_BACKEND=ast cargo test -p expressa --lib --test ast_regressions --test bytecode_conformance --offline --locked`: 412 passaram; um helper ignorado. A conformidade seleciona explicitamente cada backend e o caminho EXB.
- `cargo clippy --workspace --all-targets --offline --locked -- -D warnings`, `cargo fmt --all -- --check` e `git diff --check`: aprovados.

Foram conferidos os hashes dos fontes, binários, arquivos de referência, patch e inputs, além das contagens de amostras. O executável release final é idêntico ao congelado para os benchmarks. O patch de implementação foi conferido com `git apply --check`, aplicado em cópia temporária da referência e comparado integralmente aos fontes/testes atuais.
