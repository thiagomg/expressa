# Cópia de grafo com `clone`

`graph.lep` executa 200 cópias de um mapa contendo duas referências à mesma lista, um ciclo e uma função sem capturas. Modifica cada cópia e verifica isolamento da origem, aliases internos, ciclo e execução da função. A saída deve ser `verdadeiro`.

Este é um caso específico da API nova. Está numa subpasta para não entrar no glob dos benchmarks antigos, pois a versão anterior não implementa `clone`.

```sh
target/release/expressa benchmarks/programs/clone/graph.lep
```

A sonda `benchmarks/clone_allocations.rs` mede alocações da API capturada, depois de um aquecimento. Contadores de bytes solicitados ao alocador não são RSS, e a instrumentação não deve ser usada para comparar tempos contra o executável normal. [Implementação, metodologia e resultados](../../CLONE.md).

`sprites.lep` não usa `clone`: compara a versão anterior com a candidata em 100 quadros, 64 sprites por quadro e snapshots rasterizados. A saída é validada contra uma imagem de referência construída com retângulos. Ele não abre janela.
