# Ensino médio

As calculadoras pedem os dados e terminam após apresentar o resultado.
Nos exemplos que indicam essa opção, pressione Enter sem digitar nada para
cancelar. Entradas inválidas podem ser corrigidas sem reiniciar.
Na configuração padrão, use vírgula para decimais (`2,5`). Em juros, a taxa é
uma porcentagem por período: `2` significa 2%.
Em `seno_cosseno.lep`, mude os dados no código e execute de novo. `calc_fracoes.lep` e `compara_fracoes.lep` usam o cursor do terminal
ou da Aula para repetir entradas inválidas; os demais exemplos interativos
usam perguntas simples.

```bash
cargo run -- exemplos/escola/bhaskara.lep
```

Veja o [índice de todos os exemplos](../README.md).

| Arquivo | Matéria |
|---------|---------|
| `bhaskara.lep` | equação do 2º grau (Δ e raízes) |
| `juros.lep` | juros simples e compostos |
| `estatistica.lep` | lê números e explica a média, o valor central (mediana), os valores mais repetidos (moda) e suas contagens |
| `mdc_mmc.lep` | calcula o máximo divisor comum (MDC), o mínimo múltiplo comum (MMC) e simplifica a fração a/b |
| `cinematica.lep` | calcula movimento com velocidade constante (MRU) e com aceleração constante (MRUV) |
| `compara_fracoes.lep` | lê duas frações ou inteiros e compara seus valores |
| `calc_fracoes.lep` | lê duas frações e calcula a soma |
| `seno_cosseno.lep` | seno e cosseno no plano cartesiano |

Tabuada, fatorial/combinações e média de notas estão em `linguagem/`.

A raiz quadrada é `raiz(n)` no módulo `mat` (`importe "mat"`). `pi` é um valor do mesmo módulo (`seno_cosseno.lep`).
