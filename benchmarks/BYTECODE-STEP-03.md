# Etapa 03: argumentos diretamente da pilha

Esta etapa parte da VM `02-inline-dispatch`, aprovada pelo usuário no commit `ed58045`. O binário e as fontes dessa referência já estavam preservados e seus hashes foram conferidos antes de editar o runtime. Nenhum commit novo é criado automaticamente.

Nota posterior: a correção de parsing da [tarefa 019](../lang-tasks/019-newline-continuation.md), para listas iniciadas na linha seguinte, não faz parte dos binários medidos nesta etapa. Os resultados e snapshots abaixo continuam históricos e não foram sobrescritos.

Reteste posterior solicitado com o laptop sem uso: [duas novas rodadas e validação funcional](BYTECODE-STEP-03-RETEST.md). Os resultados abaixo permanecem históricos.

## Alteração isolada

Antes, toda instrução `Call` separava os argumentos com `Vec::split_off` e passava esse vetor ao despachante de chamadas. Nos caminhos para bytecode das três variantes avaliadas, a chamada retira o valor da função, mantém os argumentos na própria pilha e passa apenas sua quantidade. Para funções de bytecode, `drain` transfere os valores diretamente para os parâmetros no ambiente da função, sem criar um vetor temporário.

Na variante 03A, retirar a função com `Vec::remove` desloca os argumentos uma posição: continua existindo movimentação proporcional à quantidade de argumentos. Na variante 03B, `mem::replace` coloca `Nada` temporariamente na posição da função. Esse marcador é removido antes de criar o frame ou chamar uma nativa, inclusive nos caminhos de erro; assim, não é necessário deslocar os argumentos. O ganho potencial é evitar a alocação/desalocação do vetor nas chamadas com argumentos. Chamadas sem argumentos já não precisavam alocar esse vetor na versão anterior. Nas variantes 03A/03B, a mudança também altera o formato do resultado interno `Step::Call`, portanto o benchmark mede o efeito da implementação completa dessa passagem, não apenas um custo isolado de alocador.

A variante 03C mantém o marcador da 03B e acrescenta um caminho `Step::CallStack` exclusivo para funções de bytecode. O caminho `Step::Call(Value, Vec<Value>, ...)` é preservado para as demais chamadas. Isso conserva o formato de passagem anterior das nativas. O compilador Rust ainda pode gerar código nativo diferente para o despacho completo; não foi isolada a causa das diferenças de tempo entre as variantes.

Chamadas nativas, callbacks encaminhados pelas nativas e `avalie` continuam usando o caminho com vetor. Não foram alterados o compilador de bytecode, a resolução de nomes, a semântica numérica ou o backend AST.

Em erros de aridade/profundidade, os argumentos são removidos antes de propagar o erro. Em chamadas válidas, os argumentos saem da pilha antes de criar o novo frame. Isso preserva a base da pilha do chamador e o tempo de liberação de valores.

Código: [execute.rs](../src/runtime/eval/bytecode/execute.rs). Regressões: [bytecode_conformance.rs](../tests/bytecode_conformance.rs). Patches: [03A](bytecode-steps/03-stack-arguments.patch), [03B](bytecode-steps/03b-stack-arguments-no-shift.patch) e [03C](bytecode-steps/03c-stack-call-path.patch).

## Validação funcional

A suíte ganhou dois testes diferenciais/de integração, com vários programas: chamadas sem argumentos, vários argumentos, chamadas aninhadas, função retornada por outra função, ordem dos efeitos, erro na avaliação de um argumento, erros de aridade, recuperação com `se_falhar`, interação com nativas e `avalie`. O teste de profundidade agora também exercita recursão com argumento.

Um teste Unix verifica processos passados como argumentos: liberação no retorno normal, liberação após erro de aridade e preservação quando capturados por uma closure que escapou. Os dois backends são exercitados.

- **396 testes do núcleo passaram em debug e release**: 363 da biblioteca, 21 regressões e 12 testes diferenciais/de integração.
- `cargo clippy -p expressa --all-targets --offline --locked -- -D warnings` passou.
- `cargo fmt -p expressa --check` e `git diff --check` passaram.
- Os hashes dos binários, fontes, arquivos de snapshot, patches e workloads foram conferidos; as medianas publicadas foram recalculadas a partir das amostras. O binário preservado como `03c-stack-call-path` corresponde à variante 03C medida.
- A suíte precisou de acesso fora do sandbox para dois testes existentes que usam `~/.cache`. Não foi alterado o tratamento desses testes.

A validação desta etapa cobre o núcleo; não foi repetida a suíte gráfica do workspace. O backend AST não foi modificado. Nenhum commit foi criado.

## Método de medição

- Build release com dependências travadas/offline, mesmo compilador e configurações decimal/locale.
- Nove workloads: os oito anteriores e um novo [arguments.lep](programs/arguments.lep), que combina chamadas com oito argumentos e sem argumentos em 50.000 iterações. O novo workload é executado igualmente por todas as referências; não se compara sua medição com dados históricos que não o continham.
- Afinidade fixada no núcleo 0. Ordem intercalada pseudoaleatória.
- Para cada variante, ensaio principal: AST aprovado, VM02 anterior e candidata, 31 amostras por workload/motor após 3 aquecimentos (837 amostras medidas).
- Para cada variante, confirmação: VM02/candidata, mesmo protocolo com seed 1 (558 amostras medidas).
- Para 03C, um terceiro ensaio com seed 2 repetiu VM02/candidata (mais 558 amostras), pois os resultados de alguns workloads mudaram de sentido entre os primeiros ensaios.
- Todas as 4.743 amostras medidas passaram na verificação da saída exata, stderr vazio e sucesso do processo. Houve também 459 execuções de aquecimento válidas.
- CPU = usuário+sistema; RSS = pico do processo inteiro. São execuções novas, incluindo parser, compilação, execução e limpeza. Não há medição de heap isolado nem contagem instrumentada de alocações.

O hash `working_source_sha256` dos resultados identifica as fontes do candidato. O [manifesto](references/bytecode-steps.json) identifica as fontes e o binário de cada referência. Fontes, testes e binários estão preservados localmente em `target/bytecode-steps/03-stack-arguments/`, `target/bytecode-steps/03b-stack-arguments-no-shift/` e `target/bytecode-steps/03c-stack-call-path/`. Cada variante é comparada com VM02 no mesmo ensaio; as medições de uma variante não usam resultados de outro momento como denominador.

```sh
cargo build -p expressa --release --bin expressa --offline --locked
taskset -c 0 python3 benchmarks/run.py --engine ast=target/ast-reference-bd654b5/expressa-ast --engine vm02=target/bytecode-steps/02-inline-dispatch/expressa --engine vm03=target/bytecode-steps/03-stack-arguments/expressa --backend ast=ast --backend vm02=bytecode --backend vm03=bytecode --repeat 31 --warmup 3 --seed 0 --output benchmarks/results/bytecode-step-03.json
```

A confirmação retira o motor AST, usa `--seed 1` e grava um arquivo separado. Para 03B, o nome do motor é `vm03b`, o diretório é `03b-stack-arguments-no-shift` e os resultados são `bytecode-step-03b.json` / `bytecode-step-03b-confirmation.json`. Para 03C, são usados `vm03c`, `03c-stack-call-path` e os arquivos `bytecode-step-03c.json` / `bytecode-step-03c-confirmation.json`. Os dados anteriores são preservados.

## Resultados e decisão

### 03A: retirar a função deslocando argumentos

**Decisão:** Descartada. O teste de chamadas com 0/8 argumentos piorou 2–3%, closures ficaram próximas ou piores e houve regressões em outros workloads. Não há benefício consistente para manter esta variante.

Variação das medianas de CPU contra VM02, medida novamente em cada ensaio. Negativo significa menos CPU.

| Programa | Seed 0 | Seed 1 |
| --- | ---: | ---: |
| Chamadas com 0/8 argumentos | +2.9% | +2.0% |
| Soma em laço | +3.2% | +7.1% |
| Avaliação dinâmica | +1.2% | -2.1% |
| Chamadas e closures | +0.4% | +1.2% |
| Listas e mapas | +4.0% | +2.7% |
| Alocação e ciclos | +1.4% | +0.1% |
| Matrizes | -0.5% | -0.0% |
| Recursão | -3.7% | +1.6% |
| Inicialização | -4.4% | +1.1% |

Diferença das medianas de RSS em KiB (candidato − VM02):

| Programa | Seed 0 | Seed 1 |
| --- | ---: | ---: |
| Chamadas com 0/8 argumentos | +88 | +64 |
| Soma em laço | +104 | +16 |
| Avaliação dinâmica | +116 | +100 |
| Chamadas e closures | +128 | +92 |
| Listas e mapas | +128 | +112 |
| Alocação e ciclos | +148 | +40 |
| Matrizes | +152 | +72 |
| Recursão | +96 | +24 |
| Inicialização | +140 | +4 |

Dados: [ensaio principal](results/bytecode-step-03.json), [confirmação](results/bytecode-step-03-confirmation.json).

### 03B: marcador sem deslocamento

**Decisão:** Descartada. O teste de chamadas com 0/8 argumentos piorou aproximadamente 4–5%, e closures também pioraram nos dois ensaios. Evitar o deslocamento, isoladamente, não resolveu o resultado negativo.

Variação das medianas de CPU contra VM02, medida novamente em cada ensaio. Negativo significa menos CPU.

| Programa | Seed 0 | Seed 1 |
| --- | ---: | ---: |
| Chamadas com 0/8 argumentos | +3.7% | +4.9% |
| Soma em laço | +2.7% | +2.1% |
| Avaliação dinâmica | +2.1% | +1.0% |
| Chamadas e closures | +2.0% | +3.0% |
| Listas e mapas | +7.0% | +3.5% |
| Alocação e ciclos | +5.7% | +6.1% |
| Matrizes | -0.9% | -3.6% |
| Recursão | +0.9% | +0.4% |
| Inicialização | -5.1% | -0.1% |

Diferença das medianas de RSS em KiB (candidato − VM02):

| Programa | Seed 0 | Seed 1 |
| --- | ---: | ---: |
| Chamadas com 0/8 argumentos | +96 | +96 |
| Soma em laço | +112 | +56 |
| Avaliação dinâmica | +116 | +76 |
| Chamadas e closures | +148 | +84 |
| Listas e mapas | +180 | +120 |
| Alocação e ciclos | +204 | +84 |
| Matrizes | +124 | +68 |
| Recursão | +176 | +104 |
| Inicialização | +104 | +120 |

Dados: [ensaio principal](results/bytecode-step-03b.json), [confirmação](results/bytecode-step-03b-confirmation.json).

### 03C: caminho específico para bytecode

**Decisão:** Mantida para revisão. Nos três ensaios, closures consumiram aproximadamente 2–5% menos CPU. Os demais resultados foram mistos: o workload de chamadas com 0/8 argumentos piorou no primeiro ensaio e melhorou nos dois seguintes. Não se conclui ganho universal. O RSS geralmente aumentou; esta etapa não demonstra economia de memória.

Variação das medianas de CPU contra VM02, medida novamente em cada ensaio. Negativo significa menos CPU.

| Programa | Seed 0 | Seed 1 | Seed 2 |
| --- | ---: | ---: | ---: |
| Chamadas com 0/8 argumentos | +4.2% | -2.1% | -2.8% |
| Soma em laço | -3.2% | -2.1% | +0.9% |
| Avaliação dinâmica | +2.5% | -1.2% | +0.0% |
| Chamadas e closures | -5.1% | -2.4% | -2.4% |
| Listas e mapas | +4.0% | -4.8% | -1.4% |
| Alocação e ciclos | -1.2% | -0.2% | -1.1% |
| Matrizes | -1.8% | -2.7% | -6.4% |
| Recursão | -1.2% | -1.2% | -17.5% |
| Inicialização | -8.1% | -3.4% | +7.5% |

Diferença das medianas de RSS em KiB (candidato − VM02):

| Programa | Seed 0 | Seed 1 | Seed 2 |
| --- | ---: | ---: | ---: |
| Chamadas com 0/8 argumentos | +96 | -4 | +80 |
| Soma em laço | +100 | +88 | +28 |
| Avaliação dinâmica | +108 | +44 | +64 |
| Chamadas e closures | +96 | +36 | +64 |
| Listas e mapas | +152 | +96 | +20 |
| Alocação e ciclos | +88 | +44 | +88 |
| Matrizes | +124 | +64 | +112 |
| Recursão | +148 | +80 | +68 |
| Inicialização | +108 | +56 | +48 |

CPU absoluta e dispersão no último ensaio (seed 2), em ms:

| Programa | VM02 mediana | VM03C mediana | VM02 mín.–máx. | VM03C mín.–máx. |
| --- | ---: | ---: | ---: | ---: |
| Chamadas com 0/8 argumentos | 124.140 | 120.692 | 117.011–204.724 | 115.627–196.300 |
| Soma em laço | 60.081 | 60.594 | 55.964–98.237 | 54.887–97.843 |
| Avaliação dinâmica | 7.834 | 7.835 | 6.748–12.240 | 6.691–13.234 |
| Chamadas e closures | 83.523 | 81.492 | 80.364–140.144 | 77.949–139.031 |
| Listas e mapas | 67.210 | 66.273 | 63.710–112.279 | 60.670–105.715 |
| Alocação e ciclos | 8.796 | 8.695 | 7.750–14.060 | 7.434–13.807 |
| Matrizes | 4.193 | 3.926 | 3.162–7.176 | 3.142–6.493 |
| Recursão | 4.810 | 3.970 | 3.686–6.721 | 3.514–7.637 |
| Inicialização | 1.252 | 1.346 | 1.089–1.961 | 1.057–2.087 |

O executável passou de 4216704 para 4220992 bytes (+4288 bytes). Não é uma medição de flash de firmware. Essa referência foi preservada como `03c-stack-call-path`. Correções posteriores devem ser preservadas como uma nova referência antes de medir outra otimização.

Dados: [ensaio principal](results/bytecode-step-03c.json), [confirmação](results/bytecode-step-03c-confirmation.json), [terceiro ensaio](results/bytecode-step-03c-recheck.json).

Os tempos absolutos variaram entre os ensaios, inclusive na referência. Portanto, não se calculam ganhos comparando tempos de momentos diferentes. Os deltas usam apenas motores executados no mesmo ensaio. As tabelas e amostras não constituem um teste de significância estatística; não se atribui a variação a uma causa específica sem instrumentação.

RSS desktop não permite concluir economia de SRAM no Pico. Serialização, formato compacto, firmware e mudanças de resolução de variáveis permanecem fora desta etapa.
