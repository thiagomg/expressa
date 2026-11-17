# Ensino médio

As calculadoras pedem os dados e terminam após apresentar o resultado.
Nos exemplos que indicam essa opção, pressione Enter sem digitar nada para
cancelar. Entradas inválidas podem ser corrigidas sem reiniciar.
Na configuração padrão, use vírgula para decimais (`2,5`). Em juros, a taxa é
uma porcentagem por período: `2` significa 2%.
Em `regra_de_tres.lep` e `seno_cosseno.lep`, mude os dados no código e execute
de novo. `calc_fracoes.lep` e `compara_fracoes.lep` usam o cursor do terminal
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
| `regra_de_tres.lep` | porcentagem, regra de três direta e inversa |
| `progressoes.lep` | gera uma sequência somando sempre o mesmo valor (PA) ou multiplicando pelo mesmo valor (PG) |
| `estatistica.lep` | lê números e explica a média, o valor central (mediana), os valores mais repetidos (moda) e suas contagens |
| `pitagoras.lep` | escolhe hipotenusa, cateto ou distância entre pontos |
| `geometria.lep` | escolhe uma figura e calcula área ou volume |
| `mdc_mmc.lep` | calcula o máximo divisor comum (MDC), o mínimo múltiplo comum (MMC) e simplifica a fração a/b |
| `media_ponderada.lep` | lê notas de 0 a 10 e seus pesos positivos |
| `cinematica.lep` | calcula movimento com velocidade constante (MRU) e com aceleração constante (MRUV) |
| `compara_fracoes.lep` | lê duas frações ou inteiros e compara seus valores |
| `calc_fracoes.lep` | lê duas frações e calcula a soma |
| `seno_cosseno.lep` | seno e cosseno no plano cartesiano |

Tabuada, fatorial/combinações e boletim sem pesos estão em `linguagem/` e `programas/`.

A raiz quadrada é `raiz(n)` no módulo `mat` (`importe "mat"`). `pi` é um valor do mesmo módulo (`geometria.lep`).
