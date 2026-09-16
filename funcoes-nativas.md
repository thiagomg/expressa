# Expressa: Funções Nativas

Funções nativas vêm instaladas em todo programa. Não é preciso importar nada: basta chamá-las pelo nome.

Elas **não podem ser reatribuídas** (`escreva = 1` causa erro). Qualquer falha (arquivo inexistente, lista vazia, tipo errado) interrompe o programa, a menos que a chamada esteja protegida com `se_falhar`.

```text
linhas = leia_arquivo("dados.txt") se_falhar []
```

Caminhos de arquivo **relativos** são resolvidos a partir da pasta do `.lep` em execução, não do diretório de onde o comando `expressa` foi chamado.

---

## Índice

| Função | Resumo |
|--------|--------|
| [`escreva`](#escreva) | Imprime valores na saída |
| [`leia`](#leia) | Lê uma linha do teclado |
| [`numero`](#numero) | Transforma texto em número |
| [`raiz`](#raiz) | Raiz quadrada |
| [`tamanho`](#tamanho) | Quantidade de itens ou caracteres |
| [`primeiro`](#primeiro) | Primeiro elemento de uma lista |
| [`ultimo`](#ultimo) | Último elemento de uma lista |
| [`maiuscula`](#maiuscula) | Texto em letras maiúsculas |
| [`minuscula`](#minuscula) | Texto em letras minúsculas |
| [`substitua`](#substitua) | Troca trechos de um texto |
| [`separe`](#separe) | Parte um texto em lista |
| [`junte`](#junte) | Junta uma lista em um texto |
| [`limpe`](#limpe) | Remove espaços das extremidades |
| [`leia_arquivo`](#leia_arquivo) | Lê um arquivo em lista de linhas |
| [`salve_arquivo`](#salve_arquivo) | Grava uma lista de linhas em um arquivo |
| [`adicione_arquivo`](#adicione_arquivo) | Acrescenta linhas ao final de um arquivo |
| [`leia_csv`](#leia_csv) | Lê um CSV em lista de listas |
| [`salve_csv`](#salve_csv) | Grava uma lista de listas como CSV |

---

## Entrada e saída

### `escreva`

```text
escreva(valor, ...)
```

Escreve os argumentos na saída padrão, separados por um espaço, e termina com uma quebra de linha.

- Aceita **zero ou mais** argumentos. Sem argumentos, imprime só a linha vazia.
- Números inteiros aparecem sem casas decimais (`10` e não `10.0`).
- Booleanos aparecem como `verdadeiro` e `falso`.
- Listas e mapas são impressos de forma legível, por exemplo `[1, 2, 3]`.
- Retorna `nada`.

```text
escreva("Olá")
escreva("Média:", 7.5)
escreva()                  // linha em branco
```

### `leia`

```text
leia() -> texto
leia(prompt) -> texto
```

Lê uma linha da entrada padrão (teclado) e devolve o texto **sem** a quebra de linha. Espaços no começo e no fim são preservados.

- Sem argumentos, apenas espera a linha.
- Com um `prompt` (texto), imprime o prompt **sem** quebra de linha, descarrega a saída e então espera. Assim o cursor fica na mesma linha: `Nome: _`.
- Fim da entrada (Ctrl+D no Linux/macOS, Ctrl+Z no Windows) é erro: `"fim da entrada"`. Use `se_falhar` se quiser um valor padrão.

**Erros:** mais de um argumento; `prompt` não é texto; não há mais entrada.

```text
nome = leia("Qual o seu nome? ")
escreva("Olá, " + nome)

linha = leia() se_falhar ""
```

No modo `expressa debug`, a entrada do programa e os comandos do depurador compartilham o mesmo teclado.

---

## Matemática

### `numero`

```text
numero(texto) -> numero
numero(numero) -> numero
```

Converte um texto em número, para usar o que veio de `leia()` em contas.

- Aceita espaços nas pontas (`"  7 "`).
- Aceita `_` como em literais (`"1_000"`).
- O padrão de texto é **pt-BR** (`1.000,5`). Mude com `formato("en")` para en-US (`1,000.5`).
- Em pt-BR: `,` é decimal; `.` em grupos de três é milhar (`"1.000"`). Um único `.` com 1–2 casas (`"3.14"`) ainda vale (texto colado do código).
- Se o argumento já é número, devolve o mesmo valor.
- Texto que não é número é erro (`se_falhar` ajuda).

```text
idade = numero(leia("Quantos anos você tem? ")) se_falhar 0
escreva("ano que vem: " + (idade + 1))
```

**Erros:** argumento que não é texto nem número; texto que não representa um número.

### `formato`

```text
formato("pt")
formato("en")
```

Define como `numero()` lê texto e como `escreva` / `"a" + n` escrevem números. Padrão: pt-BR.

Também: `formato("pt-br")`, `formato("en-us")`, `formato("br")`, `formato("eua")`.  
Na linha de comando: `expressa --numeros en arquivo.lep` (ou `EXPRESSA_NUMEROS=en`).

Literais no código continuam com ponto: `media = 7.3`.

---

### `raiz`

```text
raiz(numero) -> numero
```

Raiz quadrada. `raiz(9)` vale `3`, `raiz(0)` vale `0`.

**Erros:** o argumento não é número; o número é negativo (`"raiz de número negativo"`). Use `se_falhar` se o valor puder ser negativo.

```text
escreva(raiz(9 + 16))            // 5
x = raiz(delta) se_falhar 0
```

---

## Coleções

### `tamanho`

```text
tamanho(valor) -> numero
```

Retorna a quantidade de elementos (lista ou mapa) ou de caracteres (texto). A contagem de texto usa caracteres Unicode, não bytes.

**Erros:** o argumento não é texto, lista nem mapa.

```text
tamanho("olá")             // 3
tamanho([10, 20, 30])      // 3
tamanho(mapa inicio
    "a" -> 1
    "b" -> 2
fim)                       // 2
```

### `primeiro`

```text
primeiro(lista) -> valor
```

Retorna o primeiro elemento da lista (o de índice `1`).

**Erros:** o argumento não é lista, ou a lista está vazia.

```text
primeiro([10, 20, 30])     // 10
primeiro([]) se_falhar 0   // 0
```

### `ultimo`

```text
ultimo(lista) -> valor
```

Retorna o último elemento da lista.

**Erros:** o argumento não é lista, ou a lista está vazia.

```text
ultimo([10, 20, 30])       // 30
```

---

## Texto

Todas as funções desta seção devolvem um **texto novo**. O valor original não é alterado.

### `maiuscula`

```text
maiuscula(texto) -> texto
```

Converte todas as letras para maiúsculas (respeita Unicode: `"olá"` vira `"OLÁ"`).

**Erros:** o argumento não é texto.

```text
maiuscula("Olá, mundo")    // "OLÁ, MUNDO"
```

### `minuscula`

```text
minuscula(texto) -> texto
```

Converte todas as letras para minúsculas.

**Erros:** o argumento não é texto.

```text
minuscula("Olá, Mundo")    // "olá, mundo"
```

### `substitua`

```text
substitua(texto, antigo, novo) -> texto
```

Troca **todas** as ocorrências de `antigo` por `novo`.

**Erros:** algum argumento não é texto.

```text
substitua("Maria Silva", "Maria", "Ana")
// "Ana Silva"

substitua("aaa", "a", "b")
// "bbb"
```

### `separe`

```text
separe(texto, separador) -> lista
```

Parte o texto em uma lista de textos.

- Se `separador` for `""` (vazio), cada caractere vira um item.
- Caso contrário, o texto é dividido em cada ocorrência do separador.
- Trechos vazios entre separadores entram na lista (`separe("a,,b", ",")` → `["a", "", "b"]`).

**Erros:** algum argumento não é texto.

```text
separe("a,b,c", ",")       // ["a", "b", "c"]
separe("ola", "")          // ["o", "l", "a"]
```

### `junte`

```text
junte(lista, separador) -> texto
```

Concatena os itens da lista, intercalando `separador`. Cada item é convertido para texto (números entram na forma usual, booleanos como `verdadeiro`/`falso`).

**Erros:** o primeiro argumento não é lista, ou o segundo não é texto.

```text
junte(["a", "b", "c"], " - ")   // "a - b - c"
junte([1, 2, 3], ",")           // "1,2,3"
junte([], ",")                  // ""
```

### `limpe`

```text
limpe(texto) -> texto
```

Remove espaços em branco (incluindo tabulações e quebras de linha) do começo e do fim. Espaços no meio do texto permanecem.

**Erros:** o argumento não é texto.

```text
limpe("  Maria Silva  ")   // "Maria Silva"
```

---

## Arquivos

Caminhos relativos partem da pasta do programa `.lep`. Caminhos absolutos (`/casa/dados.txt`) são usados como estão. Se a pasta de destino ainda não existir, `salve_arquivo`, `adicione_arquivo` e `salve_csv` tentam criá-la.

Falhas de leitura ou escrita (arquivo inexistente, permissão, etc.) são erros de execução e podem ser tratadas com `se_falhar`.

### `leia_arquivo`

```text
leia_arquivo(caminho) -> lista
```

Lê o arquivo de texto e devolve uma lista de linhas **sem** o caractere de quebra de linha. Arquivo vazio devolve `[]`.

**Erros:** `caminho` não é texto, ou o arquivo não pôde ser lido.

```text
linhas = leia_arquivo("dados.txt") se_falhar []
escreva(tamanho(linhas))
```

### `salve_arquivo`

```text
salve_arquivo(caminho, linhas)
```

Grava `linhas` no arquivo, **substituindo** o conteúdo anterior. Cada item vira uma linha, com `\n` no final.

**Erros:** `caminho` não é texto, `linhas` não é uma lista de textos, ou a gravação falhou.

```text
salve_arquivo("saida.txt", ["primeira", "segunda"])
```

### `adicione_arquivo`

```text
adicione_arquivo(caminho, linhas)
```

Acrescenta `linhas` ao **final** do arquivo. Se o arquivo não existir, ele é criado.

**Erros:** iguais aos de `salve_arquivo`.

```text
adicione_arquivo("saida.txt", ["nova linha"])
```

---

## CSV

O CSV da Expressa é simples: campos separados por vírgula, **sem** aspas nem escape. Uma vírgula dentro do próprio campo não é suportada.

Cada célula é sempre um **texto**, mesmo quando o conteúdo parece um número.

### `leia_csv`

```text
leia_csv(caminho) -> lista
```

Lê o arquivo e devolve uma lista de linhas, cada linha sendo uma lista de células.

**Erros:** `caminho` não é texto, ou o arquivo não pôde ser lido.

```text
// pessoas.csv:
// Ana,25
// Bruno,30

dados = leia_csv("pessoas.csv")
escreva(dados[1][1])       // Ana
escreva(dados[2][2])       // 30  (texto)
```

### `salve_csv`

```text
salve_csv(caminho, dados)
```

Grava `dados` (lista de listas) como CSV, **substituindo** o arquivo. Cada célula é convertida para texto; as células de uma linha são unidas por vírgula, e cada linha termina com `\n`.

**Erros:** `caminho` não é texto, `dados` não é uma lista de listas, ou a gravação falhou.

```text
salve_csv("saida.csv", [
    ["nome", "idade"],
    ["Ana", 25],
    ["Bruno", 30]
])
```

---

## Erros frequentes

| Situação | Exemplo de tratamento |
|----------|------------------------|
| Arquivo inexistente | `leia_arquivo("x.txt") se_falhar []` |
| Lista vazia | `primeiro(lista) se_falhar 0` — ou teste `tamanho(lista) == 0` antes |
| Tipo inesperado | não há coerção automática: `tamanho(10)` é erro |

O valor especial `nada` é o que funções como `escreva` e `salve_arquivo` devolvem quando só executam um efeito (imprimir, gravar). Não há literal `nada` na linguagem; ele aparece como resultado dessas chamadas.
