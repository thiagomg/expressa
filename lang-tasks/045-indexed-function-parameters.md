# 045 — Parâmetros de funções acessados por posição

Status: implementado no bytecode para revisão, com regressões de desempenho documentadas; nenhum commit automático.

## Implementação e limites

O compilador emite `LoadLocal` e `StoreLocal` para parâmetros no escopo principal da função. Nomes e posições são compartilhados pelo código compilado; cada chamada possui um vetor próprio de valores. A resolução por nome, incluindo `avalie`, acessa os mesmos valores. Não há cópia paralela a sincronizar.

Outras variáveis, referências capturadas e acessos dentro de blocos/laços continuam por nome. Isso preserva o sombreamento por variáveis de iteração e evita introduzir uma análise estática incompleta. Callbacks nativos inicializam o mesmo armazenamento. Depurador e coleta de ciclos enumeram parâmetros e bindings nomeados uma única vez.

`Env.vars` agora usa `Bindings`: operações usuais de mapa são preservadas, mas código Rust externo que depende do tipo concreto `HashMap` ou de métodos específicos precisa de ajuste. Esta consequência está documentada no embedding. Sintaxe, escopos, regras de escrita, modelo numérico e disponibilidade de `avalie` permanecem os mesmos.

## Evidência

Regressões verificam `avalie` alterando parâmetros, captura por closure, restrição de escrita de nomes externos, sombreamento em laços, callbacks nativos, mutação de listas, visibilidade no depurador e coleta de um ciclo que passa por um parâmetro. Um teste do compilador confirma a emissão dos opcodes locais e o fallback por nome em blocos.

A implementação foi medida separadamente da migração dos testes de processos. [CPU/RSS, regressões e dados brutos](../benchmarks/BYTECODE-STEP-04.md). [Patch independente](../benchmarks/bytecode-steps/04-parameter-slots.patch).

Pico 2: não implica firmware pronto nem redução comprovada de SRAM; orçamento de heap, representação compacta e validação de hardware continuam pendentes.

## Extensão pendente indicada por perf

Os [perfis após a etapa 09](../benchmarks/PERF-PRIORITIES.md) priorizam ampliar os acessos por posição, começando por parâmetros dentro de blocos/laços quando seu binding puder ser comprovado. Isso ainda não está implementado. Exige alcançar o ambiente correto e preservar sombreamento, `avalie`, closures e depuração; não basta habilitar o opcode atual em todo bloco.

O plano de implementação dessa extensão está separado na [tarefa 049](049-lexical-slot-access.md), com etapas de leitura, escrita e expansão posterior para locais. O status da entrega original desta tarefa permanece inalterado.
