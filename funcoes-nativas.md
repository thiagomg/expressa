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
| [`cls`](#cls) | Limpa a tela (`limpe_tela` também) |
| [`casa`](#casa) | Cursor no canto, sem apagar |
| [`durma`](#durma) | Espera um número de segundos |
| [`leia`](#leia) | Lê uma linha do teclado |
| [`leia_linhas`](#leia_linhas) | Lê todas as linhas da entrada padrão |
| [`eh_terminal`](#eh_terminal) | Entrada é o teclado? (`é_terminal` também) |
| [`argumentos`](#argumentos) | Lista dos valores após o `.lep` |
| [`numero`](#numero) | Transforma texto em número |
| [`formate`](#formate) | Monta texto com alinhamento (`{:<n}` `{:>n}` `{:^n}`) |
| [`raiz`](#raiz) | Raiz quadrada |
| [`aleatorio`](#aleatorio) | Inteiro ao acaso (`aleatório` também) |
| [`semente`](#semente) | Fixa a sequência de `aleatorio` |
| [`transposta`](#transposta) | Transposta |
| [`det`](#det) | Determinante 1×1–3×3 |
| [`identidade`](#identidade) | Matriz identidade |
| [`zeros`](#zeros) | Matriz de zeros |
| [`uns`](#uns) | Matriz de uns |
| [`cheia`](#cheia) | Matriz preenchida com um valor |
| [`nlinhas`](#nlinhas) | Número de linhas |
| [`ncolunas`](#ncolunas) | Número de colunas |
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

### `cls`

```text
cls()
limpe_tela()
```

Limpa a tela do terminal e coloca o cursor no canto superior esquerdo. Sem argumentos. As duas grafias são a mesma função.

No REPL, a linha `cls` (sem parênteses) também limpa a tela; `cls()` é esta função.

Na Aula, limpa o painel de saída. Quando a saída não é um terminal (pipe, arquivo), ainda assim escreve o código ANSI de limpar a tela.

Retorna `nada`.

```text
repita 3 vezes
inicio
    cls()
    escreva("contador")
fim
```

**Erros:** qualquer argumento.

Em uma animação, `cls()` a cada quadro **pisca**: a tela fica vazia um instante antes do próximo `escreva`. Use [`casa`](#casa) no laço e `cls()` só uma vez no começo.

### `casa`

```text
casa()
```

Manda o cursor para o canto superior esquerdo **sem apagar** o que já está na tela. O próximo `escreva` desenha por cima. Sem argumentos.

```text
cls()
repita 20 vezes
inicio
    casa()
    escreva(quadro)
    durma(0.1)
fim
```

Monte o quadro inteiro numa variável e dê um `escreva` só: vários `escreva` no meio do quadro ainda aparecem aos poucos.

Na Aula, o painel de saída trata `casa()` como um novo quadro (substitui o texto).

Retorna `nada`.

**Erros:** qualquer argumento.

### `durma`

```text
durma(segundos)
```

Pausa o programa pelo número de **segundos** indicado. Aceita fração: `durma(0.5)` espera meio segundo. `durma(0)` não espera.

Na Aula, o tempo-limite do programa continua valendo: uma espera longa vira `tempo esgotado`.

Retorna `nada`.

```text
escreva("3")
durma(1)
escreva("2")
durma(1)
escreva("1")
```

**Erros:** falta o argumento, valor que não é número, ou número negativo.

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

### `formate`

```text
formate(modelo, valor, ...) -> texto
```

Monta um texto a partir de um **modelo**. Cada `{}` (ou `{1}`, `{2}`, …) é substituído pelo valor correspondente. O resultado é texto: use em `escreva`, `+`, arquivos, etc.

Alinhamento (útil em tabelas). `n` é a largura em caracteres:

| no modelo | efeito |
|-----------|--------|
| `{}` | o valor, sem padding |
| `{:<n}` | encosta à **esquerda** |
| `{:>n}` | encosta à **direita** |
| `{:^n}` | **centro** |
| `{:-<n}` | esquerda, preenchido com `-` (qualquer caractere no lugar de `-`) |

Sem `<` `>` `^`, texto encosta à esquerda e número à direita: `{:8}`.

Se o valor já é mais largo que `n`, ele entra inteiro (não corta). `{1}` é o **primeiro** valor (índices começam em 1). `{{` e `}}` escrevem `{` e `}`. Números seguem o `formato` atual (`7,5` em pt).

```text
formate("Hello {:<5}!", "x")     // "Hello x    !"
formate("Hello {:-<5}!", "x")    // "Hello x----!"
formate("Hello {:^5}!", "x")     // "Hello   x  !"
formate("Hello {:>5}!", "x")     // "Hello     x!"

escreva(formate("{:<10} {:>6}", "Ana", 7.5))
escreva(formate("{:<10} {:>6}", "Bruno", 10))
```

```text
Ana             7,5
Bruno            10
```

**Erros:** o modelo não é texto; `{` sem `}`; valor a menos ou a mais; misturar `{}` e `{1}`; índice `0` (use `{1}`).

---

## Matrizes

```text
A = matriz {
    [1, 2],
    [3, 4]
}
```

Índices em 1: `A[1, 2]`. `A[1]` é a linha como lista. `A + B`, `k * A`, `A * B` (produto).
`nlinhas(A)` e `ncolunas(A)` são o tamanho; `tamanho(A)` também é o número de linhas.

### `nlinhas`

```text
nlinhas(A) -> numero
```

Quantidade de linhas. `nlinhas(matriz { [1, 2, 3], [4, 5, 6] })` vale `2`.

**Erros:** o argumento não é matriz.

### `ncolunas`

```text
ncolunas(A) -> numero
```

Quantidade de colunas. `ncolunas(matriz { [1, 2, 3], [4, 5, 6] })` vale `3`.

**Erros:** o argumento não é matriz.

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

### `zeros`

```text
zeros(n) -> matriz
zeros(linhas, colunas) -> matriz
```

Matriz só de zeros. Um argumento: quadrada `n×n`. Dois: `linhas` × `colunas`.

```text
zeros(3)        // 3×3
zeros(2, 5)     // 2 linhas, 5 colunas
```

**Erros:** falta argumento; valor que não é inteiro `>= 1`.

### `uns`

```text
uns(n) -> matriz
uns(linhas, colunas) -> matriz
```

Como `zeros`, mas preenchida com `1`.

### `cheia`

```text
cheia(linhas, colunas, valor) -> matriz
```

Retangular, todas as células iguais a `valor` (um número).

```text
cheia(2, 3, 7)
A = zeros(14, 36)
A[2, 3] = 1
```

**Erros:** não são 3 argumentos; linhas/colunas que não são inteiro `>= 1`; `valor` que não é número.

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

### `aleatorio`

```text
aleatorio(min, max) -> numero
aleatório(min, max) -> numero
```

Devolve um **inteiro** ao acaso entre `min` e `max`, inclusive. `aleatorio(1, 6)` é um dado.

As duas grafias são a mesma função. Sem `semente()`, cada execução do programa gera uma sequência diferente.

```text
dado = aleatorio(1, 6)
escreva("saiu", dado)
```

**Erros:** faltam 2 argumentos; `min` ou `max` não é inteiro; `min > max`.

### `semente`

```text
semente(n)
```

Fixa o gerador de `aleatorio`. O mesmo `n` produz a mesma sequência — útil para repetir um teste.

```text
semente(1)
escreva(aleatorio(1, 6))
```

Retorna `nada`.

**Erros:** falta o argumento, ou valor que não é inteiro.

---

## Coleções

### `tamanho`

```text
tamanho(valor) -> numero
```

Retorna a quantidade de elementos (lista, mapa ou conjunto), de caracteres (texto) ou de linhas (matriz). A contagem de texto usa caracteres Unicode, não bytes. Para colunas de matriz, use `ncolunas`.

**Erros:** o argumento não é texto, lista, mapa, conjunto nem matriz.

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
