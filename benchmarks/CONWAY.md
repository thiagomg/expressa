# Conway — AST × bytecode atuais

Primeira comparação do exemplo Conway determinístico, sem alterações no interpretador. Ambos os backends usam o **mesmo executável completo atual**, com decimal e o mesmo destino raster. [Exemplo e suas limitações](programs/conway/README.md), [runner](conway.py), [manifesto](references/conway.json).

## Resultado

Os percentuais abaixo comparam medianas de CPU por caso/rodada. Negativo significa menor consumo no bytecode; RSS é o pico do processo, não apenas heap. Não se calcula uma média global de ganhos.

Rodada 60: CPU bytecode/AST de -16.83% a -14.61%, com bytecode consumindo menos CPU em 9/9 casos. A diferença de RSS bytecode−AST vai de -36 a +212 KiB.

Rodada 61: CPU bytecode/AST de -17.19% a -14.76%, com bytecode consumindo menos CPU em 9/9 casos. A diferença de RSS bytecode−AST vai de -40 a +132 KiB.

Este é um resultado deste programa, tamanhos e configuração; não uma aceleração garantida para qualquer jogo. A redução de CPU em simulação pode orientar a próxima investigação, mas o ensaio não identifica o custo isolado de cada opcode, alocação, desenho ou chamada nativa. RSS pequeno e variável entre rodadas não é evidência suficiente de menor uso de heap.

## O que foi medido

Processos novos, incluindo leitura, parsing, compilação quando aplicável, configuração dos gráficos, gerações de aquecimento, preparação/cache de quadros no modo desenho, execução, checksum e limpeza. O processo Python controlador e a referência de correção estão fora dos contadores do interpretador. O helper C mede CPU usuário/sistema, tempo monotônico de parede e pico RSS do filho.

O destino é **raster**, sem janela, IPC ou viewer. `como_imagem` exercita desenho e rasterização/criação da imagem em memória, mas não apresentação na tela. Não há FPS, latência de entrada ou P95/P99 por frame neste relatório. Os percentis das tabelas são de execuções completas, não de frames.

`desenho` também calcula as gerações na preparação, guarda as matrizes e depois percorre os quadros. Portanto sua CPU total **não é o custo isolado da renderização**. Não se subtrai sua mediana da de simulação para tentar estimar uma fase. O cache também altera o pico de memória. Para separar fases, é necessária instrumentação própria.

Os casos ajustam gerações para executar aproximadamente 100 mil atualizações de células, permitindo repetições sem tornar 128×128 excessivamente longo. Isso não é uma curva de FPS para tamanhos diferentes. Checksum, inicialização e proporção de aquecimento variam com o tamanho.

| Lado | Gerações de execução | Gerações de aquecimento | Atualizações totais de células |
| --- | ---: | ---: | ---: |
| 32×32 | 100 | 10 | 112640 |
| 64×64 | 25 | 3 | 114688 |
| 128×128 | 6 | 1 | 114688 |

## Protocolo e proveniência

- Duas rodadas, seeds 60 e 61, com AST/bytecode e os nove casos intercalados em ordem aleatória registrada.
- 15 amostras por backend/caso/rodada, depois de três aquecimentos de processo. Total: **540 amostras medidas e 108 aquecimentos de processo**. Cada programa ainda executa suas gerações de aquecimento.
- Afinidade CPU 0; mesma máquina, executável, fonte, configuração decimal/locale e destino gráfico. Nenhuma compilação ou suíte de testes foi executada durante as amostras; edição leve de documentação continuou. Frequência, temperatura e todos os processos do SO não foram controlados.
- Todas as execuções precisam terminar com sucesso, sem stderr, e coincidir integralmente com stdout esperado. Contagem de vivas/checksum são calculados por uma referência independente em Python, com enumeração genérica dos oito vizinhos, em vez das oito leituras explícitas do exemplo.
- Um ensaio funcional prévio, seed 59 com uma amostra por combinação, validou todos os caminhos e o protocolo; ele não entra nas estatísticas publicadas.
- Arquivos e executável foram congelados antes da coleta. O runner verifica que os hashes não mudaram entre início e fim. Metadados registram hashes, ambiente, backend explícito, valores esperados e parâmetros.

Host: Linux-7.3.0-10-generic-x86_64-with-glibc2.44. CPU: Intel(R) Core(TM) i5-10210U CPU @ 1.60GHz. Compilador: rustc 1.98.1 (48a229cea 2026-09-01).

O executável usado tem 3273496 bytes e está **stripped**. Foi comprovado que corresponde exatamente à aplicação de `strip` ao release completo documentado na etapa 07, com o mesmo BuildID. Não se compara esse tamanho diretamente aos tamanhos não stripped dos relatórios anteriores. AST e bytecode usam o mesmo arquivo, então não há diferença de tamanho de executável entre os motores deste ensaio.

Fonte base: `c2de421fb0598a24dd5300de81e7420d8cdd579e`; fingerprint Rust/Cargo/fontes nativas `3b7329a5e49b6d42f4d66beaf3e006ca2e0ef77c18476159bb7851c63cc3fd02`. O exemplo adicionado à árvore de trabalho tem hash separado no manifesto. O arquivo `source.tar` preserva os fontes, testes, exemplo, runner e helper, inclusive arquivos ainda não commitados. Os binários/arquivos congelados ficam em `target/benchmarks/conway-baseline-c2de421/` como artefatos locais.

```sh
# Para repetir a referência, usar fontes e toolchain identificados no manifesto.
# Escolher um diretório NOVO para outro ensaio, preservando o original.
cargo build -p expressa --release --bin expressa --offline --locked
mkdir -p target/benchmarks/conway-novo
cp target/release/expressa target/benchmarks/conway-novo/expressa
strip target/benchmarks/conway-novo/expressa
cp benchmarks/programs/conway/conway.lep target/benchmarks/conway-novo/conway.lep
cc -O2 -std=c11 -Wall -Wextra -Werror benchmarks/measure.c \
  -o target/benchmarks/conway-novo/measure

# Repetir trocando seed/saída para 61:
taskset -c 0 python3 benchmarks/conway.py \
  --binary target/benchmarks/conway-novo/expressa \
  --program target/benchmarks/conway-novo/conway.lep \
  --helper target/benchmarks/conway-novo/measure \
  --repeat 15 --warmup 3 --seed 60 \
  --output benchmarks/results/conway-novo-60.json
```

Os comandos criam outra referência e outros arquivos de saída. Os dados publicados usam o diretório original indicado no manifesto. Preservar hashes e arquivos de cada ensaio; não sobrescrever resultados anteriores.

## CPU e memória — rodada 60

Medianas. CPU em ms e RSS em KiB (1.024 bytes). Diferenças positivas significam mais consumo no bytecode.

| Caso | CPU AST | CPU VM | Variação CPU | RSS AST | RSS VM | Diferença RSS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| simulacao-32 | 498.472 | 424.353 | -14.87% | 5296 | 5412 | +116 |
| desenho-32 | 564.272 | 470.569 | -16.61% | 6484 | 6448 | -36 |
| completo-32 | 559.421 | 474.707 | -15.14% | 5548 | 5540 | -8 |
| simulacao-64 | 515.036 | 439.811 | -14.61% | 5240 | 5452 | +212 |
| desenho-64 | 587.149 | 494.428 | -15.79% | 6868 | 6892 | +24 |
| completo-64 | 581.839 | 496.535 | -14.66% | 6024 | 6084 | +60 |
| simulacao-128 | 542.724 | 459.780 | -15.28% | 5432 | 5512 | +80 |
| desenho-128 | 615.085 | 524.985 | -14.65% | 8868 | 8980 | +112 |
| completo-128 | 630.082 | 524.064 | -16.83% | 8260 | 8328 | +68 |

### Dispersão — rodada 60

[Amostras e metadados](results/conway-60.json). Mediana [P10–P90], percentis interpolados linearmente em `(n−1)×p`. CPU = usuário+sistema por amostra; as partes e utilização estão nos dados brutos.

| Caso | Motor | CPU ms | Parede ms | Pico RSS KiB |
| --- | --- | ---: | ---: | ---: |
| simulacao-32 | ast | 498.472 [489.788–531.336] | 499.717 [490.353–532.248] | 5296 [5200–5389] |
| simulacao-32 | bytecode | 424.353 [412.904–445.208] | 424.985 [413.449–446.111] | 5412 [5235–5503] |
| desenho-32 | ast | 564.272 [550.648–575.181] | 564.870 [551.168–575.907] | 6484 [6371–6532] |
| desenho-32 | bytecode | 470.569 [464.642–492.040] | 471.137 [465.149–492.779] | 6448 [6398–6532] |
| completo-32 | ast | 559.421 [550.438–572.054] | 560.104 [551.106–572.787] | 5548 [5450–5690] |
| completo-32 | bytecode | 474.707 [465.197–489.591] | 475.378 [465.722–490.337] | 5540 [5490–5627] |
| simulacao-64 | ast | 515.036 [502.115–527.083] | 515.524 [502.673–527.822] | 5240 [5149–5439] |
| simulacao-64 | bytecode | 439.811 [425.996–453.132] | 440.448 [426.463–453.758] | 5452 [5266–5477] |
| desenho-64 | ast | 587.149 [570.989–593.035] | 587.922 [571.662–593.725] | 6868 [6813–6973] |
| desenho-64 | bytecode | 494.428 [488.626–503.212] | 496.627 [489.247–503.768] | 6892 [6806–6998] |
| completo-64 | ast | 581.839 [571.491–596.238] | 582.590 [572.120–596.977] | 6024 [5955–6098] |
| completo-64 | bytecode | 496.535 [483.984–517.536] | 497.529 [484.614–518.251] | 6084 [5994–6163] |
| simulacao-128 | ast | 542.724 [538.082–548.813] | 543.225 [538.742–549.439] | 5432 [5302–5516] |
| simulacao-128 | bytecode | 459.780 [450.721–462.124] | 460.278 [451.145–462.741] | 5512 [5417–5628] |
| desenho-128 | ast | 615.085 [608.767–630.310] | 618.701 [609.311–631.094] | 8868 [8822–8977] |
| desenho-128 | bytecode | 524.985 [511.820–534.126] | 526.316 [512.440–534.808] | 8980 [8821–9042] |
| completo-128 | ast | 630.082 [605.728–644.859] | 630.985 [606.375–645.694] | 8260 [8206–8325] |
| completo-128 | bytecode | 524.064 [509.655–540.223] | 524.652 [510.274–541.142] | 8328 [8220–8390] |

## CPU e memória — rodada 61

Medianas. CPU em ms e RSS em KiB (1.024 bytes). Diferenças positivas significam mais consumo no bytecode.

| Caso | CPU AST | CPU VM | Variação CPU | RSS AST | RSS VM | Diferença RSS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| simulacao-32 | 504.542 | 418.377 | -17.08% | 5240 | 5372 | +132 |
| desenho-32 | 565.544 | 479.384 | -15.23% | 6440 | 6400 | -40 |
| completo-32 | 575.254 | 476.380 | -17.19% | 5536 | 5528 | -8 |
| simulacao-64 | 521.658 | 434.259 | -16.75% | 5252 | 5316 | +64 |
| desenho-64 | 581.754 | 490.313 | -15.72% | 6776 | 6804 | +28 |
| completo-64 | 584.536 | 498.287 | -14.76% | 5992 | 6032 | +40 |
| simulacao-128 | 553.532 | 460.684 | -16.77% | 5368 | 5468 | +100 |
| desenho-128 | 618.754 | 526.959 | -14.84% | 8860 | 8888 | +28 |
| completo-128 | 627.361 | 532.489 | -15.12% | 8264 | 8264 | +0 |

### Dispersão — rodada 61

[Amostras e metadados](results/conway-61.json). Mediana [P10–P90], percentis interpolados linearmente em `(n−1)×p`. CPU = usuário+sistema por amostra; as partes e utilização estão nos dados brutos.

| Caso | Motor | CPU ms | Parede ms | Pico RSS KiB |
| --- | --- | ---: | ---: | ---: |
| simulacao-32 | ast | 504.542 [497.671–522.169] | 505.146 [498.229–522.855] | 5240 [5179–5387] |
| simulacao-32 | bytecode | 418.377 [411.942–424.119] | 419.009 [412.401–424.639] | 5372 [5296–5490] |
| desenho-32 | ast | 565.544 [545.771–586.566] | 566.163 [546.387–587.199] | 6440 [6332–6566] |
| desenho-32 | bytecode | 479.384 [465.291–509.042] | 479.956 [465.881–511.881] | 6400 [6314–6518] |
| completo-32 | ast | 575.254 [559.863–597.969] | 576.063 [560.611–598.796] | 5536 [5386–5660] |
| completo-32 | bytecode | 476.380 [465.275–488.014] | 477.018 [466.122–488.709] | 5528 [5352–5638] |
| simulacao-64 | ast | 521.658 [506.542–534.837] | 522.348 [507.051–535.480] | 5252 [5194–5360] |
| simulacao-64 | bytecode | 434.259 [424.731–449.726] | 434.715 [425.337–450.282] | 5316 [5236–5439] |
| desenho-64 | ast | 581.754 [569.866–594.497] | 582.547 [570.566–595.223] | 6776 [6708–6848] |
| desenho-64 | bytecode | 490.313 [483.746–499.162] | 490.945 [484.250–499.691] | 6804 [6750–6866] |
| completo-64 | ast | 584.536 [571.257–646.213] | 585.112 [571.901–649.184] | 5992 [5888–6099] |
| completo-64 | bytecode | 498.287 [488.797–531.681] | 498.916 [489.447–532.409] | 6032 [5968–6074] |
| simulacao-128 | ast | 553.532 [537.598–566.118] | 554.151 [538.098–566.803] | 5368 [5262–5473] |
| simulacao-128 | bytecode | 460.684 [454.257–487.942] | 461.221 [454.703–488.547] | 5468 [5393–5620] |
| desenho-128 | ast | 618.754 [608.939–633.257] | 619.554 [609.618–633.939] | 8860 [8723–8970] |
| desenho-128 | bytecode | 526.959 [512.721–539.267] | 527.646 [513.285–539.863] | 8888 [8803–8942] |
| completo-128 | ast | 627.361 [618.387–675.730] | 628.109 [618.996–676.554] | 8264 [8190–8375] |
| completo-128 | bytecode | 532.489 [514.882–558.210] | 533.078 [516.548–559.007] | 8264 [8183–8377] |

## Próxima medição

Para jogos com janela, a próxima peça é uma confirmação de renderização/apresentação identificada por quadro e coleta de tempos por fase com relógio monotônico. Sem isso, o retorno de `mostre()` mede envio/encaminhamento, e pode esconder fila ou trabalho do viewer. Também será preciso contabilizar CPU e memória do processo gráfico. Nenhuma API gráfica ou código do interpretador foi alterado neste ensaio.

Esses resultados são de x86_64/Linux, não do Raspberry Pi Zero 2 W. Quando o hardware for escolhido, repetir com o mesmo programa, configuração numérica, destino, saída validada e parâmetros registrados. Resultados antigos do AST aprovado são históricos; não devem ser misturados com estes tempos atuais para calcular ganhos acumulados.

## Conferência final

Contagens e medianas das 540 amostras, somas de CPU usuário/sistema, afinidade, parâmetros e hashes de programa, executável, fonte arquivada, helper e runners conferidos. Os 108 aquecimentos de processo também passaram na verificação de saída. A sintaxe do runner e `git diff --check` passaram. Nenhum arquivo Rust de produção foi alterado, nenhum commit foi criado e os arquivos que já estavam staged foram preservados no índice.
