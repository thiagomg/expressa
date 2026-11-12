# Prioridades de desempenho após a etapa 09

Novos perfis do binário congelado `09-clone-fastpath`, SHA-256 `6ac5548d05e8da11ca95022bbd111fe95bd1237049767bb78fca596c33fe3a0d`. Nenhuma alteração adicional no runtime foi feita nesta análise. [Entrega anterior](clone-fastpath/README.md), [dados brutos](results/perf-priorities-90.json), [runner](perf_priorities.py).

## Medição

Oito programas/casos × duas repetições = **16 perfis de ciclos de usuário**, backend bytecode, CPU 0, decimal/en, perf 7.3.0, `perf_event_paranoid=2`, seed 90 e frequência solicitada 997 Hz. Registros separados, sem compilações/testes concorrentes. Saídas/stderr/exit codes conferidos; nenhuma amostra perdida. Os casos foram ampliados para obter alguns segundos de execução: aritmética 10 milhões, closures 5 milhões, argumentos 2 milhões, sprites 20 mil quadros e ciclos 2 milhões de iterações. Conway usa 64×64, 150 gerações +3 de aquecimento, saída raster, nos modos simulação/desenho/completo, com contagem/checksum de referência independentes.

São perfis do processo inteiro, incluindo preparação e limpeza. O modo `desenho` do Conway também inclui a simulação usada para preencher o cache de quadros: não equivale a perfilar isoladamente o renderer. Não foram medidos FPS de janela, latência de entrada ou desempenho em Raspberry Pi. Não há comparação de velocidade de uma nova implementação nesta coleta.

## Evidência por família de símbolos

Percentuais das amostras de ciclos, faixa entre as duas execuções. Não são percentuais de ganho prometido. Somam-se apenas símbolos folha distintos dentro de cada família; código inserido pelo compilador nos chamadores permanece atribuído aos chamadores. Famílias distintas nesta tabela não se sobrepõem pelos predicados usados.

| Caso | Hashing | Busca de bindings | Alocador | Coletor explícito | `como_imagem` | `Machine::run` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| arithmetic | 31.45–33.09% | 27.88–28.08% | 0.00–0.00% | 0.16–0.42% | 0.00–0.00% | 23.05–23.52% |
| closures | 21.15–22.35% | 20.42–20.61% | 6.97–7.56% | 0.36–0.36% | 0.00–0.00% | 22.59–24.06% |
| arguments | 10.34–10.63% | 10.86–10.97% | 6.41–7.41% | 0.20–0.34% | 0.00–0.00% | 34.64–34.97% |
| sprites | 7.99–9.72% | 8.27–8.89% | 18.74–19.05% | 0.65–0.80% | 17.60–18.53% | 10.30–11.72% |
| cycles | 8.85–9.83% | 10.09–10.77% | 20.90–21.06% | 10.30–10.34% | 0.00–0.00% | 18.74–18.75% |
| conway_simulacao | 24.66–26.16% | 25.52–27.30% | 0.12–0.18% | 0.00–0.09% | 0.00–0.00% | 20.99–22.53% |
| conway_desenho | 24.65–25.44% | 25.42–25.67% | 0.51–0.71% | 0.08–0.12% | 0.88–0.91% | 21.10–23.19% |
| conway_completo | 25.15–25.64% | 24.50–26.21% | 0.64–0.83% | 0.08–0.20% | 1.03–1.12% | 21.67–22.71% |

Hashing inclui `hash_one::<&str>` e os símbolos de `SipHasher::write`. Estes últimos também podem atender outras chaves; no teste de ciclos há hashing de identidades de objetos dentro do coletor, por exemplo. Busca inclui `Bindings::{get,get_mut,...}`, `Env::get` e `Env::assignment_frame`. Alocador inclui símbolos explícitos malloc/free/calloc/realloc, seus internos e wrappers Rust; não inclui automaticamente todo o custo dos chamadores que alocam. Coletor inclui somente símbolos explícitos de `runtime::value::cycles`, excluindo hash/alocação chamados por ele. Não se interpreta a coluna do coletor como seu custo total inclusivo.

`Machine::run` contém dispatch **e trabalho dos opcodes inserido pelo compilador**. Seu percentual não demonstra que somente o dispatch consuma essa fração. Não há call chains confiáveis ou atribuição por linha nesta coleta; os relatórios são de símbolos folha.

## 1. Ampliar acessos por posição — maior potencial na VM

Hashing e resolução de bindings somam aproximadamente 60% das amostras na aritmética, 42% em closures e 50–53% nos casos Conway. Não se promete remover todo esse custo: só alguns acessos podem ser otimizados, e sua remoção pode revelar outros gargalos.

O código confirma a causa estrutural: `Op::Load` traduz o índice da tabela de nomes em uma string e chama `Env::get`; stores fazem busca de destino e nova consulta ao binding. Os slots atuais só atendem parâmetros no escopo principal da função. O compilador desabilita esses acessos dentro de blocos/laços, como documentado na [tarefa 045](../lang-tasks/045-indexed-function-parameters.md).

Próxima extensão delimitada: resolver parâmetros em blocos/laços quando sua origem for comprovada, depois locais e variáveis de iteração. No `proxima(m,n)` do Conway, os numerosos acessos a `m` e `n` dentro dos laços ainda passam por nomes. Não basta remover o bloqueio booleano do compilador: os opcodes atuais leem parâmetros do ambiente corrente; precisam alcançar o ambiente correto e respeitar sombreamento.

Preservar o mesmo armazenamento visível por `avalie`, depurador e coletor, captura por closures e as regras de escrita. Usar fallback dinâmico onde a resolução não for demonstrável. Não trocar indiscriminadamente o hasher de todos os mapas por um algoritmo sujeito a colisões controláveis. A proposta não exige mudar a linguagem.

## 2. Conversão da paleta e pixels opacos — próximo patch pequeno sugerido

No cenário de sprites, `TelaGrafica::como_imagem` representa 17,60–18,53% das amostras. Além dos perfis, foi executado `perf annotate` desse símbolo nas duas repetições. A desmontagem mostra, no loop por pixel da paleta, busca de nome e comparações de strings; no stamping, há divisões e aritmética de composição alfa. Atribuição local de instrução está sujeita a skid de amostragem: a instrução que aparece quente não mede isoladamente toda a latência da divisão anterior.

O fonte faz `rgba_de_nome(nome_cor(i))` para cada pixel. Primeira mudança pequena recomendada: tabela direta índice → RGBA, mantendo valores, transparência e fallback para índices desconhecidos. O mesmo padrão índice → nome → RGB existe em `Raster::rgb32`, mas esta coleta raster não isola o custo da apresentação de janela.

Etapa separada: caminho direto para alfa 255 em `carimbar_rgba`, copiando RGBA sem executar a mistura geral. Nesse caso, a fórmula atual resulta exatamente no pixel fonte; alfa intermediário deve continuar pela mistura existente. Preservar clipping, rotação, limiar de transparência e snapshots; validar opacos, semitransparentes e bordas. Não remover os objetos de imagem nem compartilhar o objeto mutável original para contornar o contrato de cópia.

Há um caminho inteiro para ângulo zero já implementado em `cada_pixel_do_carimbo`; não seria uma melhoria nova adicioná-lo novamente. O cenário atual usa sprites pequenos, opacos e sem rotação; precisa de casos adicionais para generalizar qualquer ganho futuro.

## 3. Reduzir alocações transitórias na API gráfica

O alocador ocupa 18,74–19,05% no cenário de sprites. Isso inclui mapas de comandos, vetores, snapshots, strings e outros objetos; perf por símbolo não identifica sozinho qual alocação é responsável por cada parcela.

O fonte oferece um candidato pequeno e verificável: `tem_chave`, `get_lista`, `get_imagem`, `get_texto`, `get_u32`, `get_f64` e `get_i32` constroem `MapKey::Texto(chave.into())` para consultas, alocando uma chave temporária. Comparar diretamente o texto da chave com um `&str` evita essa alocação, preservando a representação e mutabilidade dos mapas. Ainda será necessário medir quantas alocações e quanto CPU isso remove.

Preservar o marcador interno de recurso não clonável. Um buffer de comandos tipado e cache de comandos são propostas maiores: `ops` e seus mapas são acessíveis ao programa, portanto não se pode ignorar mutações ou manter caches sem invalidação. Não substituir a API atual nesta otimização pequena.

## 4. Baratear chamadas de função e ambientes

Em argumentos/closures, os símbolos explícitos do alocador respondem por aproximadamente 6–8%, além de custos separados de entrada/saída de frames e strings. O caminho `call_stack` cria um `Env`, materializa os parâmetros e cria o vetor de ambientes do frame. Também copia nomes/arquivos para metadados de execução e diagnóstico.

Investigar metadados compartilhados e menos vetores temporários; depois, armazenamento de frames/slots que reduza objetos de heap por chamada. Os parâmetros já usam índices e o caminho de argumentos na pilha já existe, portanto não se apresenta isso como trabalho ainda ausente. Evitar pooling indiscriminado de ambientes: closures escapadas, hooks de depuração e `avalie` podem observar identidades/valores antigos. Qualquer reutilização precisa demonstrar ausência de referências observáveis.

## 5. Coletor, índices e dispatch — prioridades condicionais

O teste específico de ciclos mostra cerca de 21% em símbolos do alocador e 10% em símbolos explícitos do coletor, além de hashing que pode pertencer à coleta. Vale investigar scratch buffers reutilizáveis e menos estruturas temporárias por coleta se jogos criarem muitos grafos cíclicos. Nos demais casos, os símbolos explícitos do coletor são pequenos; estes perfis não justificam removê-lo ou afrouxar liberação de ciclos globalmente. A coleta não foi isolada como causa da regressão de CPU observada na etapa anterior: isso exigiria comparar antes/depois especificamente.

Conway também amostra `index_get2`, validação de inteiros e conversões decimais. Depois dos acessos por posição, avaliar caminhos comuns de índice que evitem conversões redundantes, mantendo índices inteiros, limites e erros exatos. O bytecode já tem `Index2`; não precisa de um novo opcode equivalente apenas para indexar matrizes.

Superinstruções/fusão de operações podem ser avaliadas depois, mas o percentual de `Machine::run` não mede só dispatch. Antes de uma alteração grande, seria necessário localizar os opcodes e sequências responsáveis, preservando spans, checkpoints, erros e ordem de avaliação.

## Sequência sugerida

- Para uma próxima alteração pequena e fácil de revisar: conversão direta de paleta para RGBA. Medir antes/depois e checar equivalência de pixels.
- Para o maior avanço arquitetural da VM: acessos por posição, começando por parâmetros dentro de blocos/laços e respeitando seus escopos.
- Entregar cada mudança isoladamente, com CPU/RSS, contadores, alocações e regressões; os perfis acima indicam prioridades, não substituem essa comparação.

## Reprodução

```sh
taskset -c 0 python3 benchmarks/perf_priorities.py \
  --repeat 2 --seed 90 --output benchmarks/results/perf-priorities-novo-90.json

perf annotate --stdio --percent-limit 0.5 \
  -i target/perf-priorities/sprites-0.data \
  --symbol='<expressa::runtime::graf::TelaGrafica>::como_imagem'
```

Hashes do executável, fontes dos programas, runner e dados `.data` constam dos [resultados](results/perf-priorities-90.json). Os `.data` e desmontagens ficam em `target/perf-priorities`; não foram commitados. Nenhuma alteração adicional no runtime, staging ou configurações do sistema foi feita nesta investigação.
