# 044 — Testes de retenção de processos com sincronização explícita

Status: implementado para revisão; nenhum commit automático.

Os testes `discarded_process_values_keep_the_ast_cleanup_timing` e `stack_arguments_release_processes_on_return_and_arity_error` dependiam de `sleep 0.05`, criação de arquivo e `durma(0.2)`. Falharam intermitentemente no reteste anterior.

## Implementação

Os dois testes foram transferidos da integração para [eval/tests.rs](../src/runtime/eval/tests.rs), onde podem selecionar explicitamente o backend e usar o hook interno sem ampliar a API pública. Um subprocesso Rust executa uma função auxiliar do próprio binário de testes. Antes de o valor escapar da função de criação, um checkpoint confirma a conexão e a mensagem de prontidão via socket Unix local.

No checkpoint de observação, o teste exige uma resposta do filho quando ele deve permanecer vivo, ou EOF quando deveria ter sido encerrado. Um vazamento fica aguardando autorização e faz o controlador atingir o prazo de falha; um encerramento antecipado falha ao responder. Não há janela fixa para sucesso. O controlador tem prazo de 10 segundos; o filho não encerra por temporizador, evitando confundir timeout com coleta correta. Se o controlador desaparece, a conexão fecha.

Os casos preservados cobrem descarte imediato no arquivo, retenção da última expressão no corpo de laço, retorno normal, erro de aridade e captura por closure que escapou, nos dois backends. O único teste auxiliar marcado `ignore` é iniciado explicitamente com `--ignored --exact` pelos testes; os casos de regressão não foram ignorados. Não há dependência nova de Python ou shell no protocolo; usa o executável de testes e o utilitário Unix `env`.

## Validação

Os testes passaram com as suítes debug/release concorrentes, no workspace completo e em cinco repetições debug + cinco release (20 execuções dos testes principais, 100 subprocessos de sondagem). Os sockets Unix locais foram bloqueados no sandbox; a repetição autorizada fora dele passou. O prazo detecta falha/travamento e não define a duração de sucesso.

O suporte de testes não altera o executável do interpretador. Um refinamento final do prazo do auxiliar foi feito após as medições de CPU; o SHA-256 do executável release permaneceu idêntico. Fontes de medição e fontes finais estão preservadas separadamente no manifesto da etapa.

[Patch independente](../benchmarks/bytecode-steps/04-process-handshake.patch). [Relatório da etapa seguinte](../benchmarks/BYTECODE-STEP-04.md).
