# Interpretador de bytecode: implementação desktop para revisão

A etapa AST foi revisada e commitada pelo usuário em `bd654b5`. A mensagem seguinte autorizou iniciar o bytecode. A referência foi preservada **antes** das alterações: binário e arquivo completo do código-fonte em `target/ast-reference-bd654b5/`, com [manifesto rastreável](../benchmarks/references/ast-bd654b5.json). Nenhum commit novo foi criado nesta etapa.

## O que está implementado

- Compilador de todas as expressões e comandos aceitos pelo parser para instruções em memória; constantes, nomes, spans e protótipos de função ficam em tabelas compartilhadas pelo código.
- VM de pilha com frames explícitos para chamadas Expressa e chamadas diretas de `avalie`. Chamadas comuns não recursam pelo avaliador Rust/AST.
- Closures armazenam código compilado e o ambiente léxico. Corpos AST não são retidos pelas funções de bytecode; o programa AST é descartado após a compilação nas entradas normais de arquivo, módulo, REPL e avaliação dinâmica.
- `avalie` compila em tempo de execução e usa o escopo atual: pode ler/atualizar nomes permitidos, introduzir nomes que código já compilado passa a enxergar e devolver closures que sobrevivem à avaliação.
- Blocos, condicionais, curto-circuito, listas, pares, campos, índices, fatias, funções, todos os laços, controle de fluxo, atribuições e `se_falhar` têm instruções próprias. Não há uma instrução que delegue expressões arbitrárias ao AST.
- Operações numéricas, coleções, importações, entrada/saída, gráficos, processos e demais nativas reutilizam os serviços e o modelo de valores revisados.
- Checkpoints de comandos e spans preservam depuração, pilhas e origem de erros em arquivos importados. Erros desfazem pilhas temporárias/escopos; controle de fluxo atravessa handlers sem ser capturado indevidamente.
- Processos percorridos por laços são encerrados na saída normal, `pare`, `retorne`, erro ou `saia`. Resultados temporários respeitam o tempo de retenção/liberação do AST.

O backend padrão agora é **bytecode**, sem JIT. O AST continua disponível temporariamente para testes, comparação e escolha explícita:

```sh
expressa programa.lep
expressa --backend bytecode programa.lep
expressa --backend ast programa.lep
expressa --backend bytecode debug programa.lep
```

Também é possível selecionar `ExecutionOptions::backend` pela API Rust ou definir `EXPRESSA_BACKEND`. A opção `--ast` continua sendo apenas a impressão da árvore sintática. Argumentos do programa chamados `--backend` não são consumidos como opções globais depois do arquivo.

## Organização e compatibilidade

- [compiler.rs](../src/runtime/eval/bytecode/compiler.rs): emissão de código, saltos, escopos, destinos e laços.
- [execute.rs](../src/runtime/eval/bytecode/execute.rs): despacho, frames, chamadas, handlers e liberação de recursos.
- [mod.rs](../src/runtime/eval/bytecode/mod.rs): formato interno das instruções e seleção de backend.
- [Testes diferenciais](../tests/bytecode_conformance.rs): saídas, códigos de saída, erros, spans, pilhas, avaliação dinâmica e recursos.
- [API de embedding](../docs/embedding.md): seleção do backend e limites atuais.

Não foi criada uma nova sintaxe ou um novo modelo numérico. Bindings continuam sendo resolvidos dinamicamente por nome para preservar `avalie`, referências a nomes definidos posteriormente e as restrições de escrita entre funções. Otimizações de slots locais/cache de resolução ainda não foram implementadas.

O compilador evita temporários de destino em atribuições simples de nomes, atualiza bindings sem recriar a chave quando possível e elimina acumuladores comprovadamente desnecessários. Expressões em corpos de laço continuam retidas até sua substituição/fim, porque um valor `processo` tem liberação observável. Uma regressão verifica essa distinção entre arquivo e corpo de laço.

Os testes também confirmaram que blocos/funções retêm a última **expressão** ao atravessar atribuições e laços posteriores. A descrição da especificação foi alinhada a esse comportamento existente; ele não foi alterado. `avalie` e REPL continuam retornando o valor do último item, que pode ser `nada` para uma atribuição.

A API Rust recebe `ExecutionBackend` e o campo `ExecutionOptions::backend`. Internamente, o corpo de uma closure passa a ser AST ou bytecode. Os wrappers de execução existentes continuam disponíveis. Não existe ABI de arquivo de bytecode ou carregador de instruções externas nesta etapa.

## Validação

- Backend bytecode: **363 testes da biblioteca + 21 regressões AST + 10 testes diferenciais/integração**, passando em debug e release. As tabelas dos testes diferenciais exercitam vários programas em cada teste.
- Backend AST: a mesma suíte do interpretador passou com `EXPRESSA_BACKEND=ast`.
- Um teste executa 3.000 chamadas recursivas em uma thread com pilha nativa de 256 KiB; outro verifica o limite de profundidade como erro capturável.
- Nos testes unitários, o avaliador AST possui uma asserção que detecta uso acidental pelo backend bytecode. A suíte de nativas/callbacks também passa com essa guarda.
- Workspace, excluindo apenas o teste GTK descrito abaixo: **586 testes passaram, 3 ignorados existentes e 1 filtrado**. Subprocessos de teste não são contados duas vezes.
- `cargo clippy --workspace --all-targets --offline --locked -- -D warnings`, formatação e verificação de whitespace passaram.

Comando de validação do workspace:

```sh
cargo test --workspace --offline --locked --quiet -- --skip assist::tests::help_only_analyzes_on_explicit_request
```

A execução completa sem filtro falhou em `assist::tests::help_only_analyzes_on_explicit_request`, na asserção `manual completion must load suggestions`. O mesmo teste isolado falhou **também na cópia do commit AST aprovado**, em `/tmp/expressa-approved-ast-bd654b5`, com a mesma asserção. A falha é anterior à migração/neste ambiente; nenhum arquivo da interface foi alterado e o teste não foi ignorado no código/CI.

## CPU e memória

As otimizações posteriores à primeira entrega são acompanhadas em [experiências incrementais](../benchmarks/BYTECODE-OPTIMIZATIONS.md), com uma alteração por candidato, referências preservadas e resultados inclusive das alterações descartadas.

A [comparação medida](../benchmarks/AST-VS-BYTECODE.md) usa o AST aprovado e o bytecode atual, no mesmo host, em release, com entradas iguais, ordem intercalada e verificação de cada saída. São medidos CPU de usuário+sistema, tempo decorrido e RSS máximo por processo. A primeira medição exploratória também foi preservada em [dados separados](../benchmarks/results/ast-vs-bytecode-initial.json).

Os resultados não mostram ganho universal. Há ganhos claros na recursão, ganho no laço aritmético, casos próximos e regressões, principalmente na compilação repetida de pequenos textos com `avalie`. A avaliação dinâmica atualmente compila em cada chamada; não foi adicionado um cache ilimitado para esconder esse custo.

## O que ainda falta para o Pico 2

Esta entrega é uma VM desktop, não um firmware pronto para gravar. Permanecem pendentes:

- separar o núcleo de `std`, bibliotecas gráficas/processos e outros serviços desktop (035/038);
- orçamento global de heap, alocação/GC e limites de frames apropriados à placa (037);
- eliminar ou limitar o crescimento de pilha no parser/compilador e nas chamadas síncronas de nativas que reentram no interpretador; os frames explícitos já resolvem chamadas Expressa comuns, não todas essas fronteiras (036/041);
- medir decimal, tamanho do firmware, SRAM, carregamento e console na placa real (039–043).

RSS de desktop não é SRAM do Pico. Não houve build bare-metal, gravação, benchmark ou validação de hardware. A tarefa 036 permanece parcialmente concluída por esse motivo. A implementação do AST não foi removida antes de revisão/paridade estáveis.
