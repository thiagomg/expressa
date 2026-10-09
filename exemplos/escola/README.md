# Ensino médio

Substitutos de conta no caderno: mude as constantes no topo e execute de novo.

```bash
cargo run -- exemplos/escola/bhaskara.lep
```

Índice de todos os exemplos: `../README.md`.

| Arquivo | Matéria |
|---------|---------|
| `bhaskara.lep` | equação do 2º grau (Δ e raízes) |
| `juros.lep` | juros simples e compostos |
| `regra_de_tres.lep` | porcentagem, regra de três direta e inversa |
| `progressoes.lep` | PA e PG (termo geral e soma) |
| `estatistica.lep` | média, mediana, moda, amplitude |
| `pitagoras.lep` | Pitágoras e distância entre pontos |
| `geometria.lep` | áreas e volumes |
| `mdc_mmc.lep` | MDC (Euclides), MMC, simplificar fração |
| `media_ponderada.lep` | boletim com pesos |
| `cinematica.lep` | MRU e MRUV |

Tabuada, fatorial/combinações e boletim sem pesos estão em `linguagem/` e `programas/`.

A raiz quadrada é `raiz(n)` no módulo `mat` (`importe "mat"`). `pi` é um valor do mesmo módulo (`geometria.lep`).
