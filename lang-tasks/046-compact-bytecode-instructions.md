# 046 — Representação compacta das instruções em memória

Status: experimento descartado e implementação revertida após revisão e concordância do usuário. Dados e patch preservados; sem commit automático.

O texto abaixo descreve a versão experimental, não o runtime ativo. A compactação economizou 64–672 bytes por programa nos componentes contabilizados, mas aumentou CPU em vários workloads e RSS em 232–476 KiB. A etapa 04 foi mantida. O formato `.exb` poderá ser definido separadamente, sem exigir esta representação em memória.

## Escopo

Substituir o `enum Op` retido em cada instrução por um word de 32 bits, mantendo outro word para o índice do span. `Op` permanece como representação lógica do compilador e valor temporário decodificado da VM. Nenhuma árvore sintática ou enum decodificado é retido no vetor de instruções.

São 8 bytes por instrução, contra 16 no layout anterior. O word codificado usa 8 bits para tag e 24 para o operando frequente. O bit superior da tag indica o caminho excepcional para operandos maiores, que preserva seus 32 bits em uma tabela opcional. O índice de span continua com 32 bits. Chamadas usam uma tabela de pares `(argc, nome)`, ambos de 32 bits. Saltos continuam usando ordinais de instrução, preservando o backpatching e os metadados dos laços.

As tabelas auxiliares e o aumento do cabeçalho de cada Chunk fazem parte da contagem de armazenamento. O mapa excepcional é alocado somente se necessário; o acesso indireto extra de Box é deliberada para não manter o cabeçalho inteiro de um HashMap vazio em cada Chunk.

## Compatibilidade e limites

Sintaxe, números, escopos, `avalie`, callbacks e depurador permanecem os mesmos. Não se introduz limite de 24 bits para programas: índices maiores mantêm o caminho completo u32. Os limites anteriores dos índices/tabelas u32 continuam existindo.

O formato é privado e interno, não uma ABI de arquivo. As tags não são um contrato de compatibilidade entre versões. Serialização, endianness de arquivo, validação de arquivos externos, acesso em flash e firmware Pico ficam para etapas próprias.

## Evidência

Os testes cobrem ida/volta dos 72 códigos de operação, spans completos, operandos nos limites de 24/32 bits e reescrita de saltos entre os caminhos compacto/excepcional. A suíte semântica continua comparando AST e VM.

O relatório de tamanho reconstrói o layout anterior e foi conferido contra o compilador anterior real, em uma cópia temporária do commit aprovado com instrumentação apenas de testes. Não se confunde tamanho de instruções com heap total, firmware ou RSS.

[Relatório de armazenamento, CPU e RSS](../benchmarks/BYTECODE-STEP-05.md). [Patch da implementação](../benchmarks/bytecode-steps/05-compact-instructions.patch). [Instrumentação da referência](../benchmarks/bytecode-steps/05-reference-storage-probe.patch).
