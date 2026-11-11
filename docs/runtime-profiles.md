# Perfis de compilação do runtime desktop

O build padrão mantém o interpretador completo. O primeiro perfil mínimo desativa serviços opcionais em tempo de compilação, mantendo a mesma linguagem e VM bytecode. É uma etapa da [separação do núcleo e hosts](../lang-tasks/035-portable-core-and-hosts.md), ainda baseada em `std`.

## Como compilar

```sh
# Desktop completo, comportamento padrão:
cargo build -p expressa --release --bin expressa --offline --locked

# Perfil mínimo, em outro diretório para preservar os dois executáveis:
cargo build -p expressa --release --bin expressa --no-default-features \
  --target-dir target/minimal --offline --locked

target/minimal/release/expressa programa.lep
target/minimal/release/expressa compilar programa.lep -o programa.exb
target/minimal/release/expressa programa.exb

# Exemplo: acrescentar apenas processos ao perfil mínimo:
cargo build -p expressa --release --bin expressa --no-default-features \
  --features process --target-dir target/process --offline --locked
```

`--no-default-features` sozinho é o perfil mínimo; não é necessário ativar uma feature chamada `minimal`. As features Cargo são aditivas. Em builds do workspace, Aula e outros consumidores podem reativar as features padrão de Expressa. Use `-p expressa` para obter o executável mínimo isolado e confira a árvore efetiva de dependências.

## Matriz de recursos

| Componente | Feature | Completo | Mínimo |
| --- | --- | --- | --- |
| VM bytecode, decimais, coleções, closures, GC e limites atuais | Sempre | Sim | Sim |
| Lexer, AST de parsing e compilador; `avalie` no escopo atual | Sempre | Sim | Sim |
| Compilação/carregamento EXB v1 | Sempre | Sim | Sim |
| Entrada/saída padrão, argumentos, marcador de leitura | Sempre | Sim | Sim |
| Matemática, matrizes, arquivos, datas e funções do núcleo | Sempre | Sim | Sim |
| Executor AST de referência | `ast` | Sim | Não |
| REPL/editor, depurador CLI e módulo nativo `tela` | `terminal` | Sim | Não |
| Módulo `processo`, spawn/pipelines e valor `Processo` | `process` | Sim | Não |
| Módulos `cartesiano`, `tela_grafica`, `imagem`, PNG e viewers | `graphics` | Sim | Não |
| Hooks/sessões de depuração para hosts e modelos de captura | Sempre | Sim | Sim |

Features padrão: `ast`, `graphics`, `process`, `terminal`. `graphics` ativa `terminal` porque os gráficos compartilham os serviços de cor e integração do desktop. `terminal` ativa `rustyline`; `graphics` também ativa `minifb` e `png`. Nenhuma dessas três dependências está na árvore ativa do perfil mínimo. Neste host, a árvore de dependências normais passa de 54 para 9 pacotes (sem contar Expressa; dependências de build e bibliotecas do sistema não entram nessa contagem). `rust_decimal`, `stacker` e, em Unix, `libc` permanecem: o calendário local também usa libc. A exclusão do executor AST não exclui os tipos AST temporários necessários à compilação.

Os modelos de imagem/canvas e os contratos de hosts permanecem disponíveis para compatibilidade Rust. Sem `graphics`, não há registro de funções gráficas, codecs PNG, viewer ou implementações CLI que criam filhos. Isso ainda não é uma extração de todos os tipos gráficos para uma crate separada.

## Disponibilidade e compatibilidade

A sintaxe, as regras de escopo, os números, as closures e `avalie` são compartilhados. Programas que usam capacidades removidas precisam de outro perfil. O catálogo e a ajuda de funções nativas omitem os módulos desativados. `tipos()` omite `processo` quando esse tipo não é compilado; `imagem` continua como tipo de dados para hosts. `--help` lista apenas os comandos disponíveis, e comandos removidos falham com erro explícito.

`ExecutionBackend::Ast` permanece na API para permitir uma configuração uniforme, mas `is_available()` retorna `false` sem a feature `ast`. A seleção explícita pela CLI, variável de ambiente ou `ExecutionOptions` falha nesse build; não muda silenciosamente de executor. `--ast` imprime a árvore de parsing e continua disponível, pois é independente do executor AST.

Imports bare de módulos nativos removidos produzem um erro de capacidade indisponível. Para importar um arquivo local com o mesmo nome, use o caminho explícito, por exemplo `importe "./processo"`. As regras do perfil completo permanecem iguais. A verificação EXB aceita o formato comum; a disponibilidade de bibliotecas é conferida ao executar o import, inclusive em código recebido de outro perfil.

As APIs de execução capturada e hooks continuam disponíveis. `CliDebugger`, REPL e implementações CLI de hosts gráficos só são exportados com suas features. `debug_source*` e o modo EXB `Debug` devolvem erro explícito sem `terminal`. Fornecer um host gráfico a `run_with_hosts` sem `graphics` também devolve erro. Consumidores Rust que usam `Value::Processo` ou `FunctionBody::Ast` devem compilar com as features correspondentes; os builds padrão continuam incluindo ambos. Não há promessa de ABI Rust ou layout estável entre perfis.

## Validação e medição

[Testes de perfis](../tests/runtime_profiles.rs) verificam as combinações de features, catálogo/ajuda/imports, seleção de executor, CLI e os nove workloads comuns tanto de fonte como de EXB. Os testes específicos de capacidades compilam apenas quando elas estão habilitadas. A suíte de conformidade AST/bytecode requer `ast`; as regressões comuns e os testes EXB também são executados no perfil mínimo.

[Tamanhos, CPU, RSS e protocolo](../benchmarks/MINIMAL-PROFILE.md).

## Próximas etapas para Pico 2

Este perfil não é firmware nem `no_std`. Ainda usa sistema de arquivos, calendário do SO, stdio, relógio, threads para alguns serviços e crescimento de pilha com `stacker`. Mantém ajuda e instalação de exemplos. Faltam a fronteira portátil de hosts, alocação com orçamento, stacks limitadas, perfil numérico validado e integração da placa. RSS desktop não mede SRAM e o executável desktop não mede o uso de flash do firmware. A remoção do frontend não faz parte desta mudança e exigiria rever como preservar `avalie`.
