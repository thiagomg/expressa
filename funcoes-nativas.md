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
| [`escreva`](#escreva) | Imprime valores na saída (stdout) |
| [`escreva_erro`](#escreva_erro) | Imprime na saída de erro (stderr) |
| [`sair`](#sair) | Encerra o programa |
| [`leia`](#leia) | Lê uma linha do teclado |
| [`leia_linhas`](#leia_linhas) | Lê todas as linhas da entrada padrão |
| [`eh_terminal`](#eh_terminal) | Entrada é o teclado? (`é_terminal` também) |
| [`argumentos`](#argumentos) | Lista dos valores após o `.lep` |
| [`numero`](#numero) | Transforma texto em número |
| [`raiz`](#raiz) | Raiz quadrada |
| [`transposta`](#transposta) | Transposta |
| [`det`](#det) | Determinante 1×1–3×3 |
| [`identidade`](#identidade) | Matriz identidade |
| [`tamanho`](#tamanho) | Quantidade de itens ou caracteres |
| [`primeiro`](#primeiro) | Primeiro elemento de uma lista |
| [`ultimo`](#ultimo) | Último elemento de uma lista |
| [`maiuscula`](#maiuscula) | Texto em letras maiúsculas |
| [`minuscula`](#minuscula) | Texto em letras minúsculas |
| [`sem_acento`](#sem_acento) | Tira acentos e cedilha |
| [`remova`](#remova) | Tira índice/faixa de lista ou texto, ou chave de mapa |
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

### `escreva_erro`

```text
escreva_erro(valor, ...)
```

Igual a `escreva`, mas escreve na **saída de erro** (stderr). Não entra em arquivos redirecionados com `>`.

```text
escreva_erro("nome vazio")
```

Retorna `nada`.

### `sair`

```text
sair()
sair(codigo)
```

Encerra o programa na hora. Nada depois da chamada roda.

- Sem argumento, o código de saída é **0** (sucesso).
- Com um número inteiro, esse é o código (use `1` para falha, como em `exit(1)`).
- **Não** é capturado por `se_falhar`.
- No REPL, a linha `sair` (sem parênteses) continua sendo o comando que sai do interpretador; `sair()` / `sair(1)` é esta função e encerra o processo.

```text
se nome == ""
inicio
    escreva_erro("nome vazio")
    sair(1)
fim
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

### `leia_linhas`

```text
leia_linhas() -> lista
```

Lê o restante da entrada padrão até o fim e devolve uma **lista de textos** (uma linha cada, sem a quebra de linha). Entrada vazia devolve `[]`. Linhas em branco entram na lista como `""`.

- Sem argumentos.
- Depois de um `leia()`, `leia_linhas()` continua do que ainda não foi lido.
- Na Aula (diálogo), não há fim de arquivo: a chamada é um erro. Use `leia()` ou `leia_arquivo`.

```text
para linha em leia_linhas()
inicio
    escreva(linha)
fim
```

```text
cat arquivo.txt | expressa prog.lep
```

### `eh_terminal`

```text
eh_terminal() -> bool
é_terminal() -> bool
```

`verdadeiro` se a entrada padrão é o **teclado** (terminal). `falso` se veio de um pipe, de um arquivo (`< dados.txt`) ou da Aula.

Sem argumentos. As duas grafias são a mesma função.

```text
se eh_terminal()
inicio
    nome = leia("Seu nome: ")
fim
senao
inicio
    nome = leia() se_falhar ""
fim
```

### `argumentos`

Não é uma função: é uma **lista** já definida em todo programa.

```text
argumentos          // lista de textos
argumentos[1]       // primeiro valor após o .lep
```

```text
expressa grep.lep Thiago
// argumentos == ["Thiago"]
```

No REPL a lista é `[]`. Não dá para fazer `argumentos = …` (nome nativo). Os itens ainda são uma lista comum (`tamanho(argumentos)`, `contem`, etc.).

```text
busca = argumentos[1] se_falhar ""
```

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

## Matrizes

```text
A = matriz {
    [1, 2],
    [3, 4]
}
```

Índices em 1: `A[1, 2]`. `A[1]` é a linha como lista. `A + B`, `k * A`, `A * B` (produto).
`tamanho(A)` é o número de linhas; `tamanho(A[1])` o de colunas.

### `transposta`

```text
transposta(A) -> matriz
```

### `det`

```text
det(A) -> numero
```

Só 1×1, 2×2 e 3×3.

### `identidade`

```text
identidade(n) -> matriz
```

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

### `sem_acento`

```text
sem_acento(texto) -> texto
```

Devolve uma cópia sem acentos nem cedilha. **Não** muda maiúscula/minúscula.

| vira | de |
|------|-----|
| `a` / `A` | á à â ã ä |
| `e` / `E` | é è ê |
| `i` / `I` | í ì î |
| `o` / `O` | ó ò ô õ |
| `u` / `U` | ú ù û ü |
| `c` / `C` | ç |

**Erros:** o argumento não é texto.

```text
sem_acento("São Paulo")    // "Sao Paulo"
minuscula(sem_acento("OLÁ"))    // "ola"
```

### `remova`

```text
remova(lista, i) -> lista
remova(lista, inicio, fim) -> lista
remova(texto, i) -> texto
remova(texto, inicio, fim) -> texto
remova(mapa, chave) -> mapa
remova(conjunto, elemento) -> conjunto
```

Devolve uma **cópia** sem aquele pedaço. Índices começam em 1; a faixa é inclusiva (como `xs[2..3]`). No mapa, a chave some; no conjunto, o **elemento** some.

```text
xs = [10, 20, 30, 40]
xs.remova(2)           // [10, 30, 40]
xs.remova(2, 3)        // [10, 40]
"abcd".remova(2, 3)    // "ad"
pessoa = pessoa.remova("idade")
s = s.remova(:ana)
```

**Erros:** índice menor que 1; início maior que o fim; chave inexistente; mapa com 3 argumentos. Fatia/`remova` com fim além do tamanho só corta até o último.

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
