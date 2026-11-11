# 047 — Formato EXB v1 e carregamento desktop

Status: implementado para revisão; nenhum commit automático.

## Entrega

`expressa compilar programa.lep [-o programa.exb]` produz bytecode sem executar o programa. A CLI executa e depura `.exb` sem o `.lep` original. Fonte embutida preserva diagnósticos; o carregador não a analisa novamente. Importações explícitas de `.exb` usam o mesmo cache e VM das importações fonte. `avalie` continua compilando código em tempo de execução.

Formato versionado com opcodes explícitos, números decimais exatos, checksum e validação de índices, fluxo, pilhas, escopos, spans e checkpoints. Limites restringem leitura, decodificação e trabalho do verificador. A gravação da CLI usa arquivo temporário e substituição por rename após sucesso. Veja o [contrato completo](../docs/exb.md).

O enum em memória permanece com instruções de 16 bytes; a compactação rejeitada na etapa 05 não foi restaurada. Não altera a sintaxe nem a semântica da linguagem. O backend AST não executa EXB.

## Revisão e evidência

Testes em `format/tests.rs`, `tests/exb.rs` e conformidade AST/fonte/EXB cobrem roundtrip, fixture v1 fixa, arquivos malformados, limites, precisão numérica, módulos, erros, entrada, argumentos, depuração, cancelamento e execução sem fonte externa. [Resultados e protocolo de CPU/RSS](../benchmarks/EXB-V1-REVIEW.md).

## Trabalho posterior

Não implementa firmware, perfil sem frontend, execução direta da flash, bundle de módulos, assinatura/autenticidade ou orçamento global de heap. O checksum detecta corrupção; não autentica o produtor. Os limites desktop precisam de revisão para o Pico 2. Continuar nas tarefas 034–043 antes de alegar suporte à placa.
