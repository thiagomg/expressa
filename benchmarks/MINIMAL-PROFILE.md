# Etapa 07 — primeiro perfil mínimo desktop

Implementado para revisão, sem commits automáticos. O build padrão continua incluindo todos os recursos; `--no-default-features` mantém frontend, bytecode, EXB e `avalie` e exclui executor AST, processos, gráficos e terminal interativo. [Matriz e uso](../docs/runtime-profiles.md), [tarefa 048](../lang-tasks/048-minimal-desktop-runtime-profile.md), [patch de implementação](bytecode-steps/07-minimal-profile.patch).

## Resultado de tamanho

| Executável release desktop | Bytes | MiB |
| --- | ---: | ---: |
| Referência anterior, completo | 4,358,720 | 4.157 |
| Novo completo | 4,368,280 | 4.166 |
| Novo mínimo | 1,736,400 | 1.656 |

O mínimo reduz o arquivo em 2,631,880 bytes (60.25%) frente ao novo completo; frente ao completo anterior, reduz 2,622,320 bytes (60.16%). O novo completo cresce 9,560 bytes (0.22%). Mesmos perfil release, compilador e alvo; não foi usado strip ou LTO adicional para favorecer um deles. MiB = 1.048.576 bytes. O tamanho é do arquivo ELF, não da soma de bibliotecas compartilhadas nem da flash de firmware.

A árvore de dependências normais neste host cai de 54 para 9 pacotes, sem contar Expressa e sem incluir dependências de build ou bibliotecas do sistema. `rustyline`, `minifb` e `png` não entram na árvore mínima. [Árvore completa](references/profiles/full-dependencies.txt), [mínima](references/profiles/minimal-dependencies.txt). Permanecem `std`, `rust_decimal`, `stacker` e libc em Unix, entre outros requisitos transitivos.

Os relatórios `ldd` do [completo](references/profiles/full-shared-libraries.txt) e [mínimo](references/profiles/minimal-shared-libraries.txt) registram as bibliotecas vinculadas na inicialização neste host. Não inventariam bibliotecas carregadas posteriormente por `dlopen` ao abrir serviços gráficos.

## CPU e RSS

A redução do executável e do RSS é o benefício desta etapa; não há aceleração geral da VM. As quatro features foram retiradas em conjunto: esta experiência não atribui ganhos isolados a cada uma. O perfil completo também muda seu layout de código; comparar `full` com `before` revela esse efeito e seus custos. Não foi feita uma otimização deliberada do loop aritmético.

Rodada 50: CPU mínimo/completo varia de -10.83% a +2.64%; mínimo com CPU menor em 3/9 casos. RSS mínimo−completo varia de -1268 a -1056 KiB. O novo completo tem variação de RSS de +176 a +324 KiB frente à referência anterior.

Rodada 51: CPU mínimo/completo varia de -10.20% a +6.70%; mínimo com CPU menor em 4/9 casos. RSS mínimo−completo varia de -1264 a -1044 KiB. O novo completo tem variação de RSS de +84 a +192 KiB frente à referência anterior.

As tabelas preservam regressões. Tempos muito curtos, especialmente startup e matrizes, são influenciados pela inicialização e pelo SO; mudanças de sinal entre rodadas não sustentam um ganho consistente. RSS mede o processo inteiro, incluindo bibliotecas mapeadas e compilação, e não separa heap, stacks ou objetos da VM. Não equivale à SRAM da placa.

## Protocolo e reprodução

Referência: commit `1760c1967095bdd3e581709143f7a5365ba02ea4`, implementação EXB v1 aceita, binário preservado antes das alterações. [Manifesto de fontes, binários, flags e patch](references/minimal-profile.json). Os dois novos executáveis têm o mesmo fingerprint de fonte, mas conjuntos de features distintos; flags de compilação são parte da identidade do artefato. Fontes e testes completos estão nos arquivos `source.tar` locais indicados pelo manifesto.

- `before`: executável completo anterior.
- `full`: novo executável com features padrão.
- `minimal`: novo executável com `--no-default-features`.

Todos executam os mesmos nove `.lep` no backend bytecode, incluindo parsing, compilação, inicialização e encerramento. EXB foi validado funcionalmente nos dois perfis; não se misturam seus tempos de carregamento com estes tempos fonte→resultado. AST não é um motor de medição desta rodada; continua na suíte de conformidade do build completo.

Duas rodadas intercaladas, seeds 50 e 51; 51 amostras e cinco aquecimentos por caso/motor/rodada, afinidade CPU 0. Total: 2.754 amostras medidas e 270 aquecimentos. Cada amostra deve sair com sucesso e imprimir exatamente `verdadeiro`. Nenhuma compilação Rust/Clippy ou suíte de testes foi executada durante as amostras; edição leve de documentação continuou. Não se pressupõe isolamento completo do SO. Dados incluem CPU usuário/sistema, tempo de parede, pico RSS, hashes do runner/helper, executáveis/fontes/inputs e ambiente.

```sh
# Preservar o binário anterior antes de alterar fontes. Para os novos:
cargo build -p expressa --release --bin expressa --offline --locked
# Copiar target/release/expressa para a referência local 07-full-profile.
cargo build -p expressa --release --bin expressa --no-default-features --offline --locked
# Copiar target/release/expressa para a referência local 07-minimal-profile.

# Repetir com seed 51 e outra saída:
taskset -c 0 python3 benchmarks/run.py \
  --engine before=target/bytecode-steps/07-before-minimal-profile/expressa \
  --engine full=target/bytecode-steps/07-full-profile/expressa \
  --engine minimal=target/bytecode-steps/07-minimal-profile/expressa \
  --backend before=bytecode --backend full=bytecode --backend minimal=bytecode \
  --repeat 51 --warmup 5 --seed 50 \
  --output benchmarks/results/minimal-profile-50.json
```

Os diretórios congelados são artefatos locais, não criados automaticamente pelo runner. Para reconstruir a referência anterior, usar o arquivo de fonte/commit indicado no manifesto e os mesmos flags/toolchain. O patch foi conferido com `git apply --check`, aplicado a uma cópia temporária da referência, e todos os arquivos resultantes comparados aos fontes atuais.

## Comparação — rodada 50

Variação entre medianas; positivo significa mais CPU/RSS. Não se calcula média de ganhos entre programas.

| Caso | CPU completo/antes | CPU mínimo/completo | RSS completo−antes KiB | RSS mínimo−completo KiB |
| --- | ---: | ---: | ---: | ---: |
| arguments | +1.26% | +0.39% | +188 | -1128 |
| arithmetic | -0.01% | -1.70% | +176 | -1140 |
| avalie | -1.35% | +0.13% | +180 | -1156 |
| closures | -1.83% | +2.51% | +220 | -1168 |
| collections | +2.50% | +1.40% | +252 | -1152 |
| cycles | +0.59% | +2.64% | +248 | -1056 |
| matrix | -4.46% | +0.69% | +208 | -1156 |
| recursion | -2.37% | -6.71% | +324 | -1268 |
| startup | -4.03% | -10.83% | +200 | -1168 |

### Medianas e dispersão — 50

[Dados brutos](results/minimal-profile-50.json). Cada célula mostra mediana [P10–P90], com interpolação linear em `(n−1)×p`. CPU é usuário+sistema por amostra; medianas das partes não precisam somar à mediana do total.

| Caso | Caminho | CPU ms | Parede ms | Pico RSS KiB |
| --- | --- | ---: | ---: | ---: |
| arguments | before | 63.386 [60.812–66.925] | 63.929 [61.265–67.757] | 4660 [4572–4752] |
| arguments | full | 64.185 [61.137–68.609] | 64.785 [61.562–69.108] | 4848 [4764–4984] |
| arguments | minimal | 64.438 [61.555–67.967] | 64.881 [61.980–68.794] | 3720 [3652–3784] |
| arithmetic | before | 51.245 [49.057–54.707] | 51.679 [49.457–55.364] | 4680 [4580–4752] |
| arithmetic | full | 51.241 [48.696–55.113] | 51.685 [48.995–55.520] | 4856 [4728–5000] |
| arithmetic | minimal | 50.371 [48.984–54.829] | 50.708 [49.355–55.335] | 3716 [3652–3772] |
| avalie | before | 6.317 [5.642–6.786] | 6.465 [5.786–7.201] | 4676 [4588–4756] |
| avalie | full | 6.232 [5.539–6.704] | 6.370 [5.696–6.930] | 4856 [4732–4980] |
| avalie | minimal | 6.240 [5.632–6.919] | 6.399 [5.758–7.080] | 3700 [3652–3768] |
| closures | before | 61.955 [58.335–67.660] | 62.434 [58.752–68.465] | 4660 [4588–4744] |
| closures | full | 60.819 [57.895–65.730] | 61.202 [58.268–66.355] | 4880 [4792–4980] |
| closures | minimal | 62.347 [59.720–67.471] | 63.216 [60.154–68.376] | 3712 [3676–3768] |
| collections | before | 53.328 [51.619–58.446] | 53.673 [52.027–59.009] | 4716 [4640–4832] |
| collections | full | 54.660 [51.937–58.257] | 55.221 [52.367–58.832] | 4968 [4888–5080] |
| collections | minimal | 55.424 [52.969–60.067] | 55.854 [53.352–60.652] | 3816 [3716–3876] |
| cycles | before | 7.130 [6.499–7.834] | 7.307 [6.639–8.157] | 4964 [4856–5076] |
| cycles | full | 7.172 [6.471–8.054] | 7.336 [6.620–8.244] | 5212 [5112–5376] |
| cycles | minimal | 7.361 [6.498–7.873] | 7.513 [6.645–8.230] | 4156 [4064–4248] |
| matrix | before | 3.051 [2.862–3.394] | 3.172 [2.980–3.527] | 4876 [4744–4984] |
| matrix | full | 2.915 [2.778–3.402] | 3.045 [2.906–3.616] | 5084 [4944–5228] |
| matrix | minimal | 2.935 [2.679–3.213] | 3.057 [2.796–3.349] | 3928 [3848–4012] |
| recursion | before | 3.160 [2.904–3.518] | 3.303 [3.046–3.735] | 6168 [6056–6276] |
| recursion | full | 3.085 [2.818–3.374] | 3.222 [2.951–3.539] | 6492 [6316–6584] |
| recursion | minimal | 2.878 [2.702–3.284] | 3.030 [2.829–3.428] | 5224 [5164–5288] |
| startup | before | 1.068 [0.984–1.190] | 1.181 [1.094–1.387] | 4680 [4568–4760] |
| startup | full | 1.025 [0.953–1.168] | 1.142 [1.065–1.354] | 4880 [4780–4964] |
| startup | minimal | 0.914 [0.849–1.047] | 1.026 [0.966–1.216] | 3712 [3676–3800] |

## Comparação — rodada 51

Variação entre medianas; positivo significa mais CPU/RSS. Não se calcula média de ganhos entre programas.

| Caso | CPU completo/antes | CPU mínimo/completo | RSS completo−antes KiB | RSS mínimo−completo KiB |
| --- | ---: | ---: | ---: | ---: |
| arguments | +0.66% | +0.11% | +120 | -1188 |
| arithmetic | -0.92% | +1.85% | +84 | -1152 |
| avalie | -4.76% | +6.70% | +140 | -1240 |
| closures | -0.73% | +1.67% | +116 | -1216 |
| collections | -0.45% | +3.06% | +112 | -1144 |
| cycles | +1.11% | -6.07% | +120 | -1044 |
| matrix | -3.05% | -2.72% | +124 | -1204 |
| recursion | +0.13% | -7.87% | +192 | -1264 |
| startup | -5.12% | -10.20% | +112 | -1220 |

### Medianas e dispersão — 51

[Dados brutos](results/minimal-profile-51.json). Cada célula mostra mediana [P10–P90], com interpolação linear em `(n−1)×p`. CPU é usuário+sistema por amostra; medianas das partes não precisam somar à mediana do total.

| Caso | Caminho | CPU ms | Parede ms | Pico RSS KiB |
| --- | --- | ---: | ---: | ---: |
| arguments | before | 62.996 [60.640–68.870] | 63.450 [61.026–69.569] | 4784 [4656–4876] |
| arguments | full | 63.412 [61.603–66.872] | 63.914 [62.034–67.449] | 4904 [4796–4984] |
| arguments | minimal | 63.482 [60.914–69.430] | 63.872 [61.292–69.853] | 3716 [3676–3752] |
| arithmetic | before | 50.427 [48.203–55.871] | 50.879 [48.503–56.380] | 4780 [4656–4880] |
| arithmetic | full | 49.965 [48.321–54.717] | 50.423 [48.672–55.378] | 4864 [4756–4972] |
| arithmetic | minimal | 50.889 [48.475–55.440] | 51.209 [48.817–55.908] | 3712 [3652–3756] |
| avalie | before | 6.238 [5.597–6.771] | 6.374 [5.737–7.008] | 4788 [4644–4876] |
| avalie | full | 5.941 [5.558–6.480] | 6.074 [5.680–6.693] | 4928 [4796–5016] |
| avalie | minimal | 6.339 [5.625–6.558] | 6.500 [5.763–6.747] | 3688 [3672–3752] |
| closures | before | 60.743 [58.183–66.777] | 61.168 [58.522–67.383] | 4804 [4668–4900] |
| closures | full | 60.300 [58.104–65.980] | 60.704 [58.507–66.544] | 4920 [4808–5020] |
| closures | minimal | 61.310 [59.271–65.929] | 61.758 [59.697–66.407] | 3704 [3680–3776] |
| collections | before | 53.094 [51.350–58.047] | 53.466 [51.753–58.611] | 4848 [4744–4928] |
| collections | full | 52.855 [50.593–57.680] | 53.358 [51.048–58.165] | 4960 [4900–5040] |
| collections | minimal | 54.475 [52.404–58.342] | 54.872 [52.785–58.836] | 3816 [3688–3884] |
| cycles | before | 7.106 [6.504–7.690] | 7.299 [6.639–7.928] | 5116 [5032–5180] |
| cycles | full | 7.185 [6.420–7.573] | 7.372 [6.562–7.724] | 5236 [5140–5348] |
| cycles | minimal | 6.749 [6.431–7.742] | 6.899 [6.559–7.988] | 4192 [4072–4264] |
| matrix | before | 3.114 [2.923–3.380] | 3.265 [3.043–3.554] | 4972 [4916–5052] |
| matrix | full | 3.019 [2.770–3.376] | 3.153 [2.887–3.575] | 5096 [4960–5208] |
| matrix | minimal | 2.937 [2.689–3.144] | 3.062 [2.810–3.310] | 3892 [3812–3992] |
| recursion | before | 3.097 [2.883–3.607] | 3.238 [3.020–3.837] | 6312 [6192–6424] |
| recursion | full | 3.101 [2.820–3.422] | 3.240 [2.944–3.587] | 6504 [6376–6552] |
| recursion | minimal | 2.857 [2.705–3.088] | 2.988 [2.835–3.223] | 5240 [5184–5288] |
| startup | before | 1.054 [0.999–1.169] | 1.163 [1.109–1.313] | 4796 [4640–4888] |
| startup | full | 1.000 [0.928–1.095] | 1.117 [1.046–1.239] | 4908 [4768–5004] |
| startup | minimal | 0.898 [0.837–0.948] | 1.016 [0.950–1.075] | 3688 [3636–3752] |

## Validação funcional

- Workspace completo: 627 testes das suítes principais passaram; quatro helpers intencionalmente ignorados (alguns invocados por testes de subprocessos). Inclui CLI, Aula, conformidade AST/bytecode/EXB e os cinco testes de perfis.
- Perfil mínimo em release (`cargo test -p expressa --release --no-default-features --lib --tests --offline --locked`): 316 testes passaram (283 biblioteca, 21 regressões, sete EXB e cinco perfis). Não inclui testes de capacidades removidas nem a comparação com o executor AST ausente.
- 12 combinações válidas de features: Clippy de todos os targets com `-D warnings` e cinco testes de perfis em cada uma, total de 60 execuções. Os testes de perfis executam os nove workloads tanto de fonte como de EXB. [Matriz registrada](references/profiles/feature-matrix.json).
- Clippy do workspace completo, formatação e `git diff --check` aprovados.
- Hashes dos binários, arquivos de referência, patch, runner e inputs, contagens e medianas de amostras conferidos. O executável release padrão final corresponde ao completo congelado.

A suíte padrão manteve sua cobertura de capacidades desktop. Testes específicos de gráficos, processos, terminal e AST são condicionados às features respectivas. A remoção de um serviço não é simulada por sucesso silencioso: imports e comandos indisponíveis falham claramente. A sintaxe, o modelo numérico, closures e semântica de `avalie` permanecem compartilhados.

## Limites e sequência

Este executável é x86_64/Linux com `std`, não firmware Pico nem um runtime já portável. Arquivos, calendário do SO, stdio, clocks, crescimento de pilha, ajuda e exemplos continuam presentes. Dados e hooks gráficos de hosts permanecem para compatibilidade de API, mesmo sem codecs/nativas/viewers. A remoção do frontend não foi feita porque `avalie` precisa dele.

O próximo recorte da tarefa 035 é separar os serviços restantes por contratos de host e validar o núcleo com `alloc`/`no_std` ou outro runtime suportado na placa. Orçamentos de heap/stack e medições de RAM/flash no hardware continuam nas tarefas 034–043. O resultado desktop não comprova esses requisitos.
