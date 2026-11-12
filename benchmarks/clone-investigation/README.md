# Investigação dos custos de clone e referências

Investigação após a entrega da [etapa 08](../CLONE.md). O runtime da árvore principal e seu executável permaneceram inalterados. Não houve commits nem alterações no staging. As variantes abaixo são experiências isoladas, não mudanças prontas para adoção.

## Conclusões

1. A representação de imagens explica **6.502 alocações e 312.096 bytes solicitados adicionais** no programa de sprites. Cada snapshot passou a envolver um objeto alocado de 48 bytes neste host. Os pixels continuam compartilhados: esses bytes não são cópias do conteúdo RGBA. A variante que restaura somente a representação anterior remove exatamente esses custos nos dois backends.
2. A política interna de recursos aumentou o armazenamento de **todos os mapas de 32 para 40 bytes**, mesmo mapas comuns e comandos gráficos. O programa cria 6.468 mapas: 51.744 bytes adicionais acumulados são explicados por esse campo/padding. Há ainda custos pequenos de campos visíveis, vetores e registro da nativa. O aumento do pedido de memória não implica necessariamente o mesmo aumento de memória física: o alocador pode arredondar os dois tamanhos para a mesma classe.
3. **`Value` continua com 32 bytes**, alinhamento de 8. Não houve crescimento geral do enum. Nos microbenchmarks de números, o candidato e a referência anterior ficaram próximos; não se demonstrou uma regressão intrínseca da cópia de números. Restaurar `derive(Clone)` não recuperou o desempenho de forma consistente nos programas de linguagem.
4. Separar a cópia de pares em uma função fora do caminho comum permite ao compilador inserir a cópia dos outros valores diretamente no chamador. Isso foi observado na desmontagem do microbenchmark. Reduziu bastante seu custo direto, mas o ganho no programa completo foi muito menor: aproximadamente **2% de CPU na aritmética da VM**, sem ganho equivalente no AST.
5. A causa integral da regressão original de aritmética permanece **não isolada**. As instruções/caminhos gerados e o layout do executável são hipóteses; não foram medidos eventos de CPU que permitam atribuí-la a cache ou branches. A recompilação de controle também apresentou diferenças em relação ao binário preservado. Não se deve apresentar todas essas diferenças como custo inevitável da semântica.

## Método e limites

- Binários congelados `08-before-clone` e `08-clone`; fonte candidata SHA-256 `14a26d21bbf5fc543a033a8c5995e9da09390f2079db9776a08739b11e5d6472`.
- Fontes candidatos extraídos em `/tmp/expressa-clone-investigation`, com target separado do workspace principal. Mesmo perfil release, features padrão e dependências offline/locked. A cópia de controle recompila os mesmos fontes sem alterações.
- `perf stat` foi tentado e recusado pelo sistema (`perf_event_paranoid=4`). Não se alteraram permissões ou configuração do host; não há perfil por função nem contadores de cache, ciclos ou instruções executadas.
- Ensaios curtos: sete binários com AST/bytecode, mais AST histórico; três programas, 31 amostras +3 aquecimentos, seeds 80/81. **2.790 amostras e 270 aquecimentos**.
- Ensaios longos: antes/candidato/controle/pares separados, AST/bytecode; aritmética e closures com cinco vezes mais iterações; 11 amostras +2 aquecimentos, seeds 83/84. **352 amostras e 64 aquecimentos**.
- Microbenchmark direto: sete bibliotecas, três tipos, 15 amostras +3 aquecimentos, seed 82. **315 amostras e 63 aquecimentos**. Inclui clone e descarte em laço Rust, com entrada/saída protegidas por `black_box`; não é execução da linguagem.
- Sonda de alocações: sete bibliotecas × dois backends × três programas × três amostras = **126 amostras**. Mesma instrumentação da etapa 08, aquecimento interno por processo.
- Afinidade CPU 0, aritmética decimal e locale en. Medidas temporais intercaladas em ordem pseudoaleatória, processos novos, CPU usuário+sistema e RSS do processo completo. Não houve compilações ou sondas de alocação durante os ensaios temporais; consultas leves a arquivos/documentação e símbolos continuaram.
- Saída, stderr e exit status dos programas de linguagem foram conferidos em cada execução. Sprites usa rasterização e imagem de referência, sem janela. Não são FPS. Não há controle completo de frequência, temperatura ou processos externos, nem afirmação de significância estatística.

## Variantes

| Nome | Alteração em relação ao candidato | Limite |
| --- | --- | --- |
| `control` | Nenhuma; recompilação isolada | Controla parte das diferenças de build |
| `derive` | Restaura `#[derive(Clone)]` em `Value` | Volta a copiar pares recursivamente; não é correção aceitável para cadeias profundas |
| `outline` | Move a cópia iterativa de pares para helper `#[inline(never)]` | Candidata a otimização; ainda precisa da suíte completa antes de adoção |
| `raw-manual` | Restaura `Imagem` diretamente no enum, mantendo o Clone manual | Remove o contrato aprovado de identidade de imagens; apenas contrafactual |
| `raw-derive` | Imagem direta + Clone derivado | Mesmas limitações dos dois anteriores |

Todas preservam a política interna dos mapas. As variantes raw não avaliam o grafo de imagens segundo o contrato novo; sua implementação de clone de imagem serve somente para permitir a compilação. Nenhum programa comparado chama a nova nativa `clone`. Análise de capturas e travessia do grafo não são trabalho executado por esses programas.

## Alocações e layout

Sprites, backend bytecode; valores idênticos nas três repetições. AST apresenta os mesmos deltas entre variantes. Bytes pedidos são acumulados, sem metadados do malloc; não são RSS nem bytes simultaneamente vivos.

| Variante | Alloc | Realloc | Bytes pedidos | Pico delta de bytes vivos | Delta retido após limpeza |
| --- | ---: | ---: | ---: | ---: | ---: |
| before | 147949 | 6861 | 10181253 | 159283 | 0 |
| candidate | 154455 | 6863 | 10547157 | 171250 | 0 |
| control | 154455 | 6863 | 10547157 | 171250 | 0 |
| derive | 154455 | 6863 | 10547157 | 171250 | 0 |
| outline | 154455 | 6863 | 10547157 | 171250 | 0 |
| raw-manual | 147953 | 6863 | 10235061 | 168082 | 0 |
| raw-derive | 147953 | 6863 | 10235061 | 168082 | 0 |

O total adicional frente à referência é 6.506 alocações: **6.502 objetos imagem**, três chaves `_clonavel` nos recursos e um nome `clone` no ambiente nativo. O programa produz 6.400 snapshots para comandos, 100 imagens dos quadros, uma imagem inicial e uma imagem final de referência. `6.502 × 48 = 312.096` bytes explica exatamente a diferença candidato → raw. A diferença restante raw → antes é 53.808 bytes, dos quais 51.744 correspondem aos 8 bytes extras dos 6.468 mapas.

O pico medido do candidato aumenta 11.967 bytes frente à referência, muito menos que o total acumulado de pedidos: os quadros são limpos e objetos deixam de estar vivos durante a execução. Todos os cenários instrumentados retornaram ao baseline de bytes vivos após a limpeza; isso não prova ausência geral de vazamentos.

## CPU dos programas curtos

Delta percentual contra `control` dentro de cada rodada. Negativo significa menos CPU. Consultar os dados brutos para CPU absoluta, parede, RSS e dispersão. Não somar percentuais entre variantes ou sessões.

| Seed | Backend | Caso | Antes | Candidato preservado | Derive | Pares separados | Imagem direta | Imagem direta + derive |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 80 | ast | arithmetic | -2.19% | +0.98% | +0.55% | +0.27% | +1.25% | +1.75% |
| 80 | ast | closures | -1.47% | -1.64% | -3.31% | -1.30% | -2.61% | +0.01% |
| 80 | ast | sprites | -0.45% | +1.97% | +0.15% | +1.95% | -4.51% | +5.07% |
| 80 | bytecode | arithmetic | -4.02% | -3.24% | -1.27% | -1.12% | -3.62% | -4.86% |
| 80 | bytecode | closures | -1.74% | +2.22% | +0.03% | -0.61% | +0.26% | +0.63% |
| 80 | bytecode | sprites | -4.43% | -0.36% | -3.79% | -9.43% | -3.94% | -3.73% |
| 81 | ast | arithmetic | -2.48% | -0.74% | +0.03% | -0.66% | -0.40% | +0.55% |
| 81 | ast | closures | -2.13% | +0.44% | -2.27% | -2.72% | -2.33% | +0.94% |
| 81 | ast | sprites | -2.22% | -2.11% | +0.91% | -2.33% | -2.69% | +2.51% |
| 81 | bytecode | arithmetic | -2.23% | -1.29% | -0.83% | -2.46% | -0.36% | -0.03% |
| 81 | bytecode | closures | -1.10% | -0.82% | +0.40% | -3.55% | -0.05% | +1.00% |
| 81 | bytecode | sprites | -4.17% | +0.23% | -1.07% | -1.64% | -2.63% | -2.35% |

A imagem direta com Clone manual reduz CPU de sprites nas duas rodadas e backends em relação ao controle. Isso mede o efeito conjunto da representação, incluindo alocação, referências, empréstimos e código gerado; não separa o tempo exclusivo do alocador. A variante raw+derive piora o AST de sprites apesar de remover as mesmas alocações: menos alocações não garante por si só menor CPU.

## Programas cinco vezes mais longos

CPU mediana em ms. As colunas finais comparam pares separados à recompilação de controle. Maior duração reduz o peso da inicialização, mas ainda não isola execução de parsing/limpeza.

| Seed | Backend | Caso | Antes | Candidato preservado | Controle | Pares separados | Δ contra controle |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| 83 | ast | arithmetic | 312.275 | 318.175 | 318.431 | 318.614 | +0.06% |
| 83 | ast | closures | 367.391 | 363.099 | 369.442 | 360.876 | -2.32% |
| 83 | bytecode | arithmetic | 244.691 | 249.541 | 248.283 | 242.938 | -2.15% |
| 83 | bytecode | closures | 292.107 | 293.648 | 292.142 | 288.560 | -1.23% |
| 84 | ast | arithmetic | 309.116 | 317.604 | 315.440 | 316.811 | +0.43% |
| 84 | ast | closures | 361.155 | 362.512 | 363.198 | 361.308 | -0.52% |
| 84 | bytecode | arithmetic | 241.724 | 243.871 | 245.312 | 240.389 | -2.01% |
| 84 | bytecode | closures | 284.422 | 289.341 | 287.635 | 286.020 | -0.56% |

Na VM, pares separados usa cerca de 2,0–2,2% menos CPU em aritmética e 0,6–1,2% menos em closures frente ao controle. AST aritmético permanece aproximadamente igual; não há aceleração geral demonstrada. As regressões não desaparecem uniformemente ao mudar somente a estratégia de Clone.

## Cópia direta no host

CPU mediana em ms: dez milhões de cópias/descarte de número ou referência de lista; três milhões de texto. Não confundir a redução deste laço com um ganho de igual proporção no interpretador.

| Variante | Número | Lista | Texto |
| --- | ---: | ---: | ---: |
| before | 106.640 | 110.549 | 59.766 |
| candidate | 106.113 | 113.165 | 60.536 |
| control | 107.358 | 112.951 | 61.172 |
| derive | 105.470 | 111.271 | 59.473 |
| outline | 68.672 | 44.355 | 58.976 |
| raw-manual | 107.230 | 113.829 | 60.382 |
| raw-derive | 105.903 | 110.411 | 60.238 |

Com `objdump -d -C` nos executáveis da sonda, `clone_paths::main` contém uma chamada a `Value::clone` no controle; na variante de pares separados não contém essa chamada, pois o caminho comum é inserido no chamador. Essa observação explica a oportunidade do microbenchmark, não prova o comportamento de cada call site da VM.

A soma dos símbolos out-of-line de `Value::clone` passa de 8.760 bytes no candidato/controle para 2.112 em pares separados, além do helper separado. Essa soma exclui cópias inseridas nos chamadores e não representa contagem de instruções executadas.

## Comparação nova com AST histórico

Nesta investigação o binário AST aprovado em `bd654b5` foi realmente reexecutado, junto aos demais. Não se usaram seus tempos antigos como denominador. A referência é o AST revisado anterior à VM, não a versão anterior às correções da primeira revisão. [Proveniência](../references/ast-bd654b5.json).

VM candidata preservada contra AST histórico; CPU menor é favorável à VM. RSS é pico do processo inteiro, em KiB. Estes três casos não permitem concluir sobre todos os programas.

| Seed | Caso | CPU AST ms | CPU VM ms | Δ CPU VM | RSS AST | RSS VM |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 80 | arithmetic | 59.439 | 50.593 | -14.88% | 4560 | 4760 |
| 80 | closures | 69.770 | 63.136 | -9.51% | 4532 | 4780 |
| 80 | sprites | 15.896 | 14.836 | -6.67% | 4944 | 5168 |
| 81 | arithmetic | 58.042 | 49.861 | -14.09% | 4544 | 4772 |
| 81 | closures | 67.844 | 60.588 | -10.70% | 4536 | 4840 |
| 81 | sprites | 14.635 | 13.879 | -5.17% | 4940 | 5120 |

A VM candidata usa 14–15% menos CPU em aritmética, 9–11% menos em closures e 5–7% menos em sprites nesta comparação, com RSS maior nesses três casos. A investigação não reexecutou recursão, `avalie` ou Conway; seus resultados anteriores continuam nos relatórios originais.

## Próximos passos sugeridos

1. Testar e entregar separadamente a extração da cópia de pares do caminho comum. É uma mudança pequena que visa preservar o contrato e tem evidência de ganho modesto na VM; validar cadeias profundas e a suíte completa antes de adotar.
2. Rever onde armazenamos a política de recursos, para não aumentar cada mapa comum. A proteção precisa sobreviver à edição/remoção do campo visível e a `remova`; remover a marca interna não é uma solução aceitável.
3. Investigar uma representação dos snapshots/comandos que reduza objetos temporários. É necessário preservar isolamento da origem e dos snapshots; compartilhar diretamente o objeto mutável original só para eliminar a alocação violaria o contrato.
4. Compartilhamento físico de textos permanece uma proposta separada. Não foi implementado nem avaliado nesta investigação; não há evidência de que resolveria a regressão aritmética.

## Reprodução e artefatos

Pré-requisito: artefatos congelados descritos na etapa 08 e AST histórico preservado em `target/ast-reference-bd654b5`. Os scripts não usam rede nem escrevem no runtime principal. O build usa um checkout temporário e seu próprio target.

```sh
python3 benchmarks/clone-investigation/prepare.py
taskset -c 0 python3 benchmarks/clone-investigation/measure.py \
  --seed 80 --output benchmarks/results/clone-investigation-novo-80.json
# Repetir com seed 81 e outra saída.

# Depois dos builds e ensaios temporais; não executar compilação em paralelo com medições.
taskset -c 0 python3 benchmarks/clone-investigation/diagnostics.py \
  --output benchmarks/results/clone-investigation-diagnostics-novo.json

taskset -c 0 python3 benchmarks/clone-investigation/measure_long.py \
  --long --repeat 11 --warmup 2 --seed 83 \
  --output benchmarks/results/clone-investigation-long-novo-83.json
# Repetir com seed 84 e outra saída.
```

- [Driver/build das variantes](prepare.py), [ensaios curtos](measure.py), [longos](measure_long.py), [diagnósticos](diagnostics.py), [sonda direta](clone_paths.rs), [layout](layout.rs).
- Dados: [80](../results/clone-investigation-80.json), [81](../results/clone-investigation-81.json), [83 longo](../results/clone-investigation-long-83.json), [84 longo](../results/clone-investigation-long-84.json), [alocações/layout/microbenchmark](../results/clone-investigation-diagnostics.json).
- [Manifesto e hashes desta investigação](manifest.json). Patches, executáveis, bibliotecas, sondas e desmontagem locais ficam em `target/clone-investigation`. Os patches experimentais também são incluídos no manifesto para revisão/reprodução.
- Validação: saídas dos programas, contagens/medianas, hashes e sintaxe Python conferidos; sondas Rust compiladas. Não se executou novamente toda a suíte do runtime: seus fontes não foram alterados. Nenhuma variante foi adotada ou commitada.

## Entrega posterior

A investigação acima registra o estado anterior à adoção de qualquer variante. A separação de pares foi posteriormente aplicada e medida com `perf` na [etapa 09](../clone-fastpath/README.md). O validador histórico usa os artefatos congelados, permitindo conferir seus dados após novas alterações no runtime.
