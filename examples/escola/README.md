# Exemplos Expressa

Cada arquivo é um programa completo. Na raiz do repositório:

```bash
cargo run -- examples/turma.lep
```

Programas que leem o teclado (`saudacao.lep`, `teste.lep`):

```bash
cargo run -- examples/saudacao.lep
```

| Arquivo | O que mostra |
|---------|----------------|
| `media.lep` | laço, `se`, lista, `escreva` |
| `erros.lep` | `se_falhar` (arquivo, divisão, índice) |
| `saudacao.lep` | `leia`, `limpe`, `separe`, `maiuscula` |
| `tabuada.lep` | função + `para i de 1 ate 10` |
| `fatorial.lep` | recursão (`fatorial`, `combinacoes`) |
| `filtra.lep` | função que recebe função (filtrar / mapear / reduzir) |
| `closures.lep` | função que devolve função |
| `receita.lep` | mapa + lista de chaves, escala uma receita |
| `poema.lep` | texto: `tamanho`, fatia, `contem`, `substitua`, `junte` |
| `turma.lep` | boletim com funções `media`, `maximo`, `situacao` |
| `palindromo.lep` | inverter string no braço |
| `ascii.lep` | triângulo e histograma com `repita` |
| `caixa.lep` | recibo de padaria, desconto |
| `diario.lep` | `salve_arquivo`, `adicione_arquivo`, `leia_arquivo` |
| `contatos.lep` | agenda em CSV |
| `lib/matematica.lep` | biblioteca (`soma`, `media`, `potencia`, …) |
| `usa_matematica.lep` | `importe` com e sem namespace |

### Ensino médio — coloque os dados no topo e rode

Substitutos de conta no caderno: mude as constantes e execute de novo.

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
| `fatorial.lep` | arranjos / C(n, k) |
| `tabuada.lep` | tabuada |
| `turma.lep` | notas e situação |

A raiz quadrada é a função nativa `raiz(n)`. Não há `π` nativo: `geometria.lep` define `pi` no começo.

`diario.lep` e `contatos.lep` gravam em `examples/saida/` (pasta ignorada pelo git).
