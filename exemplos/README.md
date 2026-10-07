# Exemplos Expressa

Cada arquivo é um programa completo. Na raiz do repositório:

```bash
cargo run -- exemplos/linguagem/media.lep
```

Programas que leem o teclado (`texto/saudacao.lep`, `programas/calculadora.lep`):

```bash
cargo run -- exemplos/texto/saudacao.lep
```

Pasta = para que o arquivo existe. Ordem de leitura sugerida: `linguagem/` → `texto/` → `programas/` → `escola/` / `unix/` / `jogos/`.

| Pasta | Papel |
|-------|--------|
| `linguagem/` | construtos: laço, função, importe, erro |
| `texto/` | string, fatia, desenho |
| `programas/` | mini app de ponta a ponta |
| `escola/` | conta de caderno (ensino médio) |
| `unix/` | `cat` / `grep` / `wc` |
| `jogos/` | Termo, Conway, adivinhe o número |
| `lib/` | o que outro exemplo importa |

### `linguagem/`

| Arquivo | O que mostra |
|---------|----------------|
| `media.lep` | laço, `se`, lista, `escreva` |
| `tabuada.lep` | função + `para i de 1 ate 10` |
| `fatorial.lep` | recursão (`fatorial`, `combinacoes`) |
| `erros.lep` | `se_falhar` (arquivo, divisão, índice) |
| `filtra.lep` | função que recebe função (filtrar / mapear / reduzir) |
| `closures.lep` | função que devolve função |
| `usa_matematica.lep` | `importe "../lib/matematica"` com e sem namespace |

`lib/matematica.lep` é a biblioteca (`soma`, `media`, `potencia`, …).

### `texto/`

| Arquivo | O que mostra |
|---------|----------------|
| `saudacao.lep` | `leia`, `limpe`, `separe`, `maiuscula` |
| `poema.lep` | `tamanho`, fatia, `contem`, `substitua`, `junte` |
| `palindromo.lep` | inverter string no braço |
| `ascii.lep` | triângulo e histograma com `repita` |

### `programas/`

| Arquivo | O que mostra |
|---------|----------------|
| `caixa.lep` | recibo de padaria, desconto |
| `receita.lep` | mapa + lista de chaves, escala uma receita |
| `turma.lep` | boletim com `media`, `maximo`, `situacao` |
| `diario.lep` | `salve_arquivo`, `adicione_arquivo`, `leia_arquivo` |
| `contatos.lep` | agenda em CSV |
| `calculadora.lep` | `avaliar` + `leia` |
| `frutas.lep` | matriz + tabela |

`diario.lep` e `contatos.lep` gravam em `programas/saida/` (ignorada pelo git).

### `unix/`

| Arquivo | O que mostra |
|---------|----------------|
| `cat.lep` | concatena arquivos |
| `grep.lep` | filtra linhas (`contem`) |
| `wc.lep` | conta linhas, palavras e caracteres |
| `lib/linha_cmd.lep` | opções e argumentos da linha de comando |

### `escola/`

Substitutos de conta no caderno: mude as constantes e execute de novo. Detalhe em `escola/README.md`.

A raiz quadrada é `raiz(n)` no módulo `mat` (`importe "mat"`). `pi` é um valor do mesmo módulo (`escola/geometria.lep`).
