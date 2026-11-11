# Etapa 05: instruções compactas em memória

**Decisão após revisão: experimento descartado e implementação revertida, com concordância do usuário.** O runtime voltou ao estado anterior à etapa 05, preservando os parâmetros por posição da etapa 04. Os resultados, patches, fontes e binários experimentais abaixo permanecem preservados como histórico; não descrevem a implementação ativa. Nenhum commit foi criado.

O armazenamento das instruções diminuiu, mas os benchmarks não mostraram uma melhora geral de recursos do processo. [Tarefa e escopo](../lang-tasks/046-compact-bytecode-instructions.md).

## Representação

Cada instrução retida no código passou de **16 para 8 bytes**, com dois campos u32:

| Campo | Conteúdo |
| --- | --- |
| `encoded` | 7 bits de opcode, 1 bit de caminho excepcional e 24 bits de operando frequente |
| `span` | Índice completo de 32 bits na tabela de posições do fonte |

As chamadas usam uma tabela auxiliar de pares `(argc, nome)`, ambos u32. Operandos acima de 24 bits ficam em uma tabela opcional por ordinal de instrução, preservando o intervalo u32 anterior. O mapa excepcional só é alocado quando necessário; Box evita embutir seu cabeçalho inteiro em todos os Chunks. Os programas medidos não usam essa tabela excepcional; seu overhead de buckets não entra na contagem abaixo.

O compilador mantém `Op` como representação lógica temporária; a VM decodifica uma instrução ao executá-la. Os enums decodificados não são retidos no vetor. Os saltos continuam em ordinais de instrução, sem reescrever offsets de spans ou laços. O código experimental de `encoding.rs` está preservado no [patch da implementação](bytecode-steps/05-compact-instructions.patch), com integração pequena no compilador e no executor; esse módulo foi removido do runtime ativo.

**Este não é um formato serializado.** Não há arquivo de bytecode, ABI estável de tags, loader de arquivos externos ou firmware Pico nesta etapa. Não foram alterados sintaxe, escopos, números, `avalie` ou políticas de execução.

## Contagem de armazenamento

A contagem inclui cada fonte de benchmark e seus protótipos de função aninhados. O layout anterior foi reconstruído com os mesmos opcodes lógicos e a capacidade real de um vetor construído com a mesma sequência de push. Todos os números do predecessor foram conferidos contra o **compilador anterior real**, em uma cópia temporária do commit `82b5129`, com instrumentação somente de testes.

A tabela soma a capacidade solicitada para os vetores de instruções, a nova tabela de chamadas e os cabeçalhos Rust de cada Chunk. Não soma buffers dos pools comuns de spans/nomes/constantes, Rc, metadados do alocador, módulos importados ou código criado por `avalie`. Portanto, não representa todo o bytecode, todo o heap ou o RSS.

| Programa | Instruções / Chunks | Antes, bytes | Compacto, bytes | Redução |
| --- | ---: | ---: | ---: | ---: |
| Chamadas com 0/8 argumentos | 64 / 3 | 2128 | 1456 | 31.6% |
| Soma em laço | 26 / 1 | 688 | 496 | 27.9% |
| Avaliação dinâmica | 48 / 1 | 1200 | 752 | 37.3% |
| Chamadas e closures | 44 / 3 | 1744 | 1264 | 27.5% |
| Listas e mapas | 53 / 1 | 1200 | 752 | 37.3% |
| Alocação e ciclos | 32 / 1 | 688 | 496 | 27.9% |
| Matrizes | 31 / 1 | 688 | 496 | 27.9% |
| Recursão | 35 / 2 | 1120 | 864 | 22.9% |
| Inicialização | 11 / 1 | 432 | 368 | 14.8% |

Os componentes contabilizados ficaram **14,8–37,3% menores**. Os programas são pequenos (11–64 instruções, incluindo funções); a economia absoluta é de **64–672 bytes** por programa nesses componentes. Não se espera traduzir esses bytes diretamente em páginas residentes liberadas. Programas maiores e firmware precisam de medição própria.

Os resultados detalham também bytes usados pelos vetores, tamanho unitário e tabela de chamadas. [Dados de armazenamento](results/bytecode-step-05-code-storage.json). [Instrumentação do compilador anterior](bytecode-steps/05-reference-storage-probe.patch).

## Método de desempenho

- Referência: `05-before-compact-instructions`, preservada antes das alterações, com o compilador e interpretador anteriores.
- Candidato: `05-compact-instructions`.
- Duas rodadas, seeds 30 e 31, 51 amostras por programa/motor após cinco aquecimentos. Nove workloads e dois motores: **1.836 amostras medidas e 180 aquecimentos**.
- Build release/offline/locked, mesmas configurações decimal/locale, afinidade no núcleo 0 e ordem intercalada. Nenhuma compilação ou suíte de testes concorrente do agente durante a medição.
- CPU = usuário+sistema; RSS = pico do processo inteiro. Inclui inicialização, parsing, compilação, execução e limpeza. Todas as amostras e aquecimentos passaram na verificação da saída exata, stderr vazio e sucesso do processo.
- Os deltas comparam motores dentro do mesmo ensaio. Não há controle de toda a atividade do host ou de sua frequência/temperatura. Nenhum ganho global ou significância estatística é assumido.

## CPU

Medianas em ms; positivo significa mais CPU no candidato.

| Programa | Antes, seed 30 | Compacto, seed 30 | Variação | Antes, seed 31 | Compacto, seed 31 | Variação |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Chamadas com 0/8 argumentos | 128.921 | 131.738 | +2.2% | 130.130 | 134.839 | +3.6% |
| Soma em laço | 100.374 | 101.390 | +1.0% | 102.065 | 102.697 | +0.6% |
| Avaliação dinâmica | 11.508 | 12.205 | +6.1% | 11.601 | 12.467 | +7.5% |
| Chamadas e closures | 123.384 | 125.562 | +1.8% | 125.326 | 128.122 | +2.2% |
| Listas e mapas | 108.174 | 109.873 | +1.6% | 108.729 | 111.396 | +2.5% |
| Alocação e ciclos | 13.577 | 13.648 | +0.5% | 13.720 | 13.703 | -0.1% |
| Matrizes | 5.532 | 5.413 | -2.2% | 5.558 | 5.415 | -2.6% |
| Recursão | 5.752 | 5.664 | -1.5% | 5.770 | 5.719 | -0.9% |
| Inicialização | 1.897 | 1.788 | -5.7% | 1.916 | 1.790 | -6.6% |

Os testes de chamadas/coleções consumiram aproximadamente **1,6–3,6% mais CPU**; aritmética, **0,6–1,0% mais**. `avalie` repetido consumiu **6,1–7,5% mais**, incluindo o custo da codificação durante a compilação. Recursão melhorou cerca de 0,9–1,5%; matrizes e inicialização são curtas e não sustentam conclusões fortes. Não há melhoria universal de velocidade.

Dispersão do segundo ensaio, mínimo–máximo de CPU em ms:

| Programa | Antes | Compacto |
| --- | ---: | ---: |
| Chamadas com 0/8 argumentos | 122.723–136.791 | 128.228–142.621 |
| Soma em laço | 94.044–108.851 | 97.794–118.000 |
| Avaliação dinâmica | 10.592–14.692 | 11.413–13.907 |
| Chamadas e closures | 117.798–136.236 | 124.439–138.598 |
| Listas e mapas | 102.976–120.246 | 104.957–121.010 |
| Alocação e ciclos | 13.020–15.699 | 13.140–16.429 |
| Matrizes | 5.437–6.722 | 5.172–8.312 |
| Recursão | 5.427–7.445 | 5.574–7.309 |
| Inicialização | 1.760–2.426 | 1.743–2.448 |

## RSS

Medianas em KiB; diferenças = candidato − referência.

| Programa | Antes, seed 30 | Compacto, seed 30 | Diferença | Antes, seed 31 | Compacto, seed 31 | Diferença |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Chamadas com 0/8 argumentos | 4540 | 4896 | +356 | 4624 | 4856 | +232 |
| Soma em laço | 4552 | 4896 | +344 | 4616 | 4896 | +280 |
| Avaliação dinâmica | 4540 | 4888 | +348 | 4620 | 4892 | +272 |
| Chamadas e closures | 4552 | 4888 | +336 | 4644 | 4884 | +240 |
| Listas e mapas | 4600 | 4944 | +344 | 4720 | 4964 | +244 |
| Alocação e ciclos | 4896 | 5236 | +340 | 4936 | 5224 | +288 |
| Matrizes | 4740 | 5116 | +376 | 4824 | 5116 | +292 |
| Recursão | 6064 | 6540 | +476 | 6212 | 6532 | +320 |
| Inicialização | 4540 | 4852 | +312 | 4620 | 4896 | +276 |

O RSS aumentou **312–476 KiB** na primeira rodada e **232–320 KiB** na segunda. Isso contrasta com a redução dos pequenos vetores de código e impede apresentar a mudança como economia de memória total. Não foi isolada a contribuição de páginas de código, mapeamento de bibliotecas, compilação e alocador; não se atribui o aumento a uma única causa.

O executável passou de 4229824 para 4241240 bytes (+11416 bytes). Tamanho do executável não é RSS nem tamanho de firmware Pico.

## Validação e revisão

- **405 testes do núcleo passaram** em debug e release: 372 biblioteca, 21 regressões e 12 diferenciais/integração. Um auxiliar de subprocesso permanece intencionalmente ignore no harness e é executado explicitamente pelos testes.
- **Workspace completo: 608 testes passaram**, quatro marcados ignore, nenhum filtrado. Subprocessos auxiliares não são contados duas vezes.
- Backend AST: a mesma suíte do núcleo foi executada com seleção explícita. A representação compacta pertence ao bytecode; não foi criado outro motor.
- Os testes novos verificam os 72 códigos de operação, operandos nos limites de 24/32 bits, spans completos e backpatching entre representações normal/excepcional. O relatório de armazenamento é outro teste.
- `cargo clippy --workspace --all-targets --offline --locked -- -D warnings`, formatação e whitespace passaram. A exceção localizada de Clippy para Box<HashMap> documenta a intenção de reduzir o cabeçalho comum de cada Chunk.

A implementação foi **descartada e revertida**. A economia de 64–672 bytes por programa nos componentes contabilizados não compensou os custos observados de CPU e RSS. Os números não justificam classificá-la como melhoria geral de CPU/RAM ou como pronta para deploy no Pico. A serialização `.lep` → `.exb` continua pendente e poderá definir um formato de arquivo compacto independentemente da representação executada pela VM.

## Artefatos e reprodução

- [Manifesto e fingerprints](references/bytecode-step-05.json). Binários, fontes e testes preservados em `target/bytecode-steps/05-before-compact-instructions/` e `05-compact-instructions/`.
- [Patch da implementação](bytecode-steps/05-compact-instructions.patch).
- [Rodada seed 30](results/bytecode-step-05-30.json) e [seed 31](results/bytecode-step-05-31.json): amostras individuais, CPU de usuário/sistema, parede, utilização, RSS e hashes de binários/entradas/ferramentas.
- [Contagem de armazenamento e referência real](results/bytecode-step-05-code-storage.json).

Para reproduzir o experimento, aplique o patch em um checkout separado da referência `82b5129`. Os comandos de compilação e contagem abaixo descrevem essa versão experimental; o teste `compiled_workload_storage_report` não existe no runtime ativo após a reversão. Os binários já preservados continuam utilizáveis para repetir as medições.

```sh
cargo build -p expressa --release --bin expressa --offline --locked
taskset -c 0 python3 benchmarks/run.py --engine before=target/bytecode-steps/05-before-compact-instructions/expressa --engine compact=target/bytecode-steps/05-compact-instructions/expressa --backend before=bytecode --backend compact=bytecode --repeat 51 --warmup 5 --seed 30 --output benchmarks/results/bytecode-step-05-30.json
EXPRESSA_CODE_SIZE_CSV=/tmp/expressa-step05-storage.csv cargo test -p expressa --offline --locked --lib encoding::tests::compiled_workload_storage_report --quiet
```

A confirmação usa seed 31 e outro arquivo de saída. Os dados anteriores não foram sobrescritos.

## Validação da reversão

Os arquivos Rust e testes foram comparados byte a byte com o snapshot `05-before-compact-instructions`. A otimização de parâmetros da etapa 04 e a correção de parsing permanecem presentes. O patch experimental foi retirado com aplicação reversa, sem alterar os dados históricos.

- Binário release reconstruído com o mesmo SHA-256 da referência anterior: `b1866984ad399910c8d290e751dd6845fa3f148c31035ee07a89072ef5f23c93`. Não foi necessário repetir benchmarks de um executável idêntico.
- Núcleo: **402 testes passaram em release e na repetição debug** (369 biblioteca, 21 regressões, 12 diferenciais/integração), com um auxiliar intencionalmente ignorado no harness principal.
- Na primeira execução debug, o teste preexistente `runtime::repl::canal::tests::explicit_shutdown_discards_queued_work_and_quit_reports_exit_code` falhou ao tentar enfileirar trabalho após `saia(7)`: `a sessão parou; aperte Reiniciar`. O teste passou isoladamente e na repetição da suíte completa. Essa falha pontual foi registrada; o REPL e o teste não foram alterados nesta reversão.
- `cargo clippy -p expressa --all-targets --offline --locked -- -D warnings`, formatação e whitespace passaram.
- Nenhum commit foi criado; a mudança de código da etapa 05 não permanece na árvore de trabalho.
