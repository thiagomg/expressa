# 023 — Document copying, aliasing, and binding immutability together

Status: contrato implementado para revisão, incluindo `clone`, análise conservadora de capturas e identidade compartilhada de imagens. Sem commits automáticos.

Priority/type: Decision · observed behavior.

## Implementation record

Registro histórico: o comportamento anterior de cópia/aliasing foi documentado na fase AST. A entrega de `clone` e identidade de imagens está registrada ao final deste arquivo e em [docs/clone.md](../docs/clone.md).

## Problem

Texts behave as values; lists, maps, and matrices share storage; collection-producing operations often make shallow copies; matrix row reads copy. argumentos cannot be rebound but its list elements can change.

## Reproduction / example

```text
argumentos[0] = "changed"
escreva(argumentos)
```

## Proposed direction

Implementar o contrato aprovado abaixo, preservando a distinção entre compartilhamento de objetos, substituição de bindings e valores independentes. As regras propostas não devem ser apresentadas como comportamento já implementado.

## Acceptance criteria

Assignment, parameter passing, concatenation, removal, matrix rows, nested objects, and argumentos are covered consistently in docs and tests.

## Pico 2 implications and dependencies

Pico 2: explicit alias rules are prerequisites for safe memory optimizations.

## Relevant code

- [src/runtime/value.rs](../src/runtime/value.rs)
- [src/runtime/eval.rs:1399](../src/runtime/eval.rs#L1399)

## Scope

O contrato de comportamento abaixo foi aprovado e implementado para revisão, com classificação conservadora de funções conforme [docs/clone.md](../docs/clone.md). Manter implementação e medições revisáveis, sem commits automáticos. Efeitos de imagem e otimização do armazenamento físico de textos permanecem fora desta entrega.

## Contrato aprovado — referência, clone, textos e imagens

1. Listas, mapas, conjuntos, matrizes e imagens compartilham identidade de objeto na atribuição e passagem de argumentos. Não há move implícito. Reatribuir um binding não reatribui os outros bindings que apontavam ao mesmo objeto.
2. `clone(valor)` produz uma cópia independente das coleções mutáveis alcançáveis. A travessia é recursiva, preserva ciclos e cria apenas um correspondente para cada objeto original: aliases internos continuam sendo aliases internos da cópia.
3. Números, booleanos e `nada` preservam seus valores. Não precisam de uma identidade nova.
4. Textos têm semântica de valor. Atribuições, argumentos e clones podem compartilhar armazenamento imutável. Escrita parcial continua permitida como substituição do texto no destino, respeitando suas permissões de escrita; não modifica outros valores de texto. Se o destino estiver em uma coleção compartilhada, substituir seu elemento será observado pelos aliases dessa coleção.
5. Uma **função sem capturas** pode ser clonada compartilhando código imutável. Uma função com capturas impede a clonagem, inclusive quando está aninhada em uma coleção. Essa regra substitui as propostas anteriores de rejeitar toda função ou compartilhar qualquer closure encontrada durante a cópia. Parâmetros e locais não são capturas; dependências de bindings externos, inclusive imutáveis, são capturas.
6. Mapas podem ser marcados como não clonáveis. O nome de marcador proposto é `:_clonavel = falso`. Janelas devem receber essa classificação ao serem criadas. A restrição também é verificada em mapas aninhados.
7. Módulos e processos não são clonáveis, diretamente ou dentro de coleções.
8. Imagens seguem a identidade compartilhada das coleções: `b = a` compartilha a imagem; `c = clone(a)` produz outro objeto imagem. Modificações em `a` são observadas por `b`, mas não por `c`. Operações existentes que retornam outra imagem não passam a modificar a origem silenciosamente. Novas operações de efeito devem documentar se modificam ou retornam outro objeto; nenhum nome de API de efeito foi definido ainda.
9. COW é uma otimização possível dos buffers de pixels: objetos imagem independentes podem compartilhar pixels até uma modificação exigir separação. Referências ao mesmo objeto imagem continuam observando suas alterações. Não aplicar separação de forma que uma atribuição comum se torne uma cópia independente.
10. `como_imagem(tela)` deve produzir uma imagem independente das alterações posteriores na tela, respeitando a distinção entre dados de imagem e recurso de janela não clonável.
11. Ao encontrar um valor não clonável, falhar sem modificar o original nem publicar uma cópia parcial. O diagnóstico deve identificar o caminho até o valor incompatível quando possível.
12. Não introduzir COW de coleções como pré-requisito. Primeiro estabelecer correção do contrato; compartilhamento de dados imutáveis e futuras otimizações devem preservar seus resultados observáveis.

### Exemplos do comportamento proposto

```text
a = [[1]]
b = a
c = clone(a)
b[0][0] = 9
escreva(a)  // [[9]]
escreva(c)  // [[1]]

texto = "oi"
outro = texto
texto[0] = "O"
escreva(outro)  // oi
```

Os exemplos com `clone` estão cobertos pelos testes da implementação.

### Detalhamento técnico registrado antes da implementação (histórico)

- Especificar a classificação de funções nativas, recursão e avaliação dinâmica (`avalie`) na análise de capturas. Reter um ambiente na representação atual não basta para afirmar que a função usa seus bindings. A classificação não pode ignorar acesso dinâmico ao ambiente.
- Consolidar o nome do marcador e as regras de leitura/alteração, incluindo sua relação com a identidade nativa das janelas. A conversa aprovou a capacidade de marcar mapas; não definiu se o marcador pode ser removido ou modificado depois.
- Definir diagnósticos, limites de travessia, limpeza de cópias parciais, tratamento de ciclos e integração com o coletor. Evitar depender de recursão nativa ilimitada.
- Cobrir atribuição, parâmetros, coleções aninhadas, aliases, ciclos, funções, recursos, escrita de textos em destinos compartilhados e imagens em AST/bytecode. Medir CPU, memória e alocações separadamente de mudanças no jogo. COW não implica ganho automático.

## Entrega para revisão

[Contrato implementado, limitações e compatibilidade](../docs/clone.md). [Testes](../tests/clone.rs). [Desempenho antes/depois](../benchmarks/CLONE.md). A classificação recusa chamadas indiretas não comprovadas e operações de consulta dinâmica do ambiente; não confunde ausência de prova com captura comprovada. O marcador adotado é `:_clonavel` bool; mapas comuns podem alterá-lo, mas recursos nativos mantêm uma marca interna que não é removida por edição do campo. Efeitos de imagem e otimização do armazenamento de textos não foram adicionados.
