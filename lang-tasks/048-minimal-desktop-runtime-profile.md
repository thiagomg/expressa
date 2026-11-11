# 048 — Primeiro perfil mínimo do runtime desktop

Status: implementado para revisão; nenhum commit automático. Primeiro recorte da tarefa 035.

## Entrega

Features `ast`, `graphics`, `process` e `terminal`, habilitadas por padrão, permitem retirar o executor AST e os serviços desktop selecionados. `--no-default-features` mantém bytecode, EXB, parser/compilador e `avalie`. O perfil padrão preserva CLI/Aula e suas bibliotecas.

A exclusão ocorre em compilação: registro e dispatch de nativas, ajuda dos módulos, código de processos, codecs/viewers e dependências opcionais. Serviços indisponíveis produzem erros claros. Tipos de parsing, modelos de captura e hooks de hosts continuam presentes quando necessários à API compartilhada.

[Comandos, matriz de dependências e limites](../docs/runtime-profiles.md). [Medições comparando a referência anterior, o novo completo e o novo mínimo](../benchmarks/MINIMAL-PROFILE.md). Testes de compatibilidade cobrem combinações de features e os nove workloads em fonte/EXB.

## Pendente

A tarefa 035 não termina aqui: separar todos os hosts, eliminar pressupostos de SO, suportar `no_std`/`alloc` e construir firmware ainda exige trabalho. Este perfil não comprova que o runtime cabe no Pico 2. Sem alteração de sintaxe, modelo numérico ou semântica de `avalie`.
