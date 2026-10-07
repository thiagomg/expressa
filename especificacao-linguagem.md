<!--
[ID]: # (169813dc-920f-46c3-a229-92ad27cc32a8)
[DATE]: # (2026-08-10 02:27:16.000)
[AUTHOR]: # (Thiago Massari Guedes)
[TAGS]: # ()
-->
# Expressa: Especificação da Linguagem
Expressa - linguagem de programaçao em português
Extensao de arquivos: .lep
---

yyy1

Linguagem orientada a iniciantes, com palavras em **português brasileiro**, sintaxe simples e consistente.  
Todo bloco (`inicio`/`fim` ou `{`/`}`) é uma expressão e retorna o valor da última expressão. As duas formas são equivalentes; o par de abertura deve combinar com o de fechamento (`inicio` com `fim`, `{` com `}`).

---

## 1. Tipos Básicos

| Tipo     | Exemplos                          |
|----------|-----------------------------------|
| `numero` | `10`, `3.14`, `-8`, `0` (decimal; `formato("float")` liga IEEE-754) |
| `texto`  | `"olá"`, `"123"`                  |
| `bool`   | `verdadeiro`, `falso`             |

Tipos compostos: **lista**, **par**, **mapa**, **conjunto** e **matriz**.

---

## 2. Variáveis e Escopo

- Declaração: apenas `nome = valor`
- Reatribuição permitida
- Escopo de bloco: variáveis criadas dentro de `inicio...fim` (ou `{...}`) só existem dentro dele
- Funções podem **ler** variáveis de fora, mas **não podem modificá-las**

```text
x = 10

inicio
    y = 20
    escreva(x)        // 10
fim

// y não existe aqui
```

---

## 3. Comentários

```text
// comentário de uma linha

/*
  comentário
  de várias linhas
*/

/// documentação da definição seguinte
soma = funcao(a, b) { a + b }
```

`///` nas linhas imediatamente acima de uma definição (`nome = …` ou `importe`) vira a ficha daquele nome (`ajuda(soma)`). `////` é um comentário comum.

A **primeira linha** do arquivo pode ser um shebang Unix (`#!…`). A Expressa ignora essa linha. `#` **não** é comentário em qualquer outro lugar.

As funções nativas estão documentadas em `docs/nativas/*.lep` (mesmo formato `///`). `ajuda("escreva")` e `ajuda("matriz::zeros")` devolvem essa ficha.

```text
#!/usr/bin/env expressa
```

---

## 4. Operadores

**Aritméticos:** `+` `-` `*` `/` `%`  
**Atribuição composta:** `+=` (número, texto, lista), `-=` (número) — `alvo += v` é `alvo = alvo + v`.  
**Comparação:** `==` `!=` `>` `<` `>=` `<=`  
**Lógicos:** `e` `ou` `nao`  
**Par:** `chave -> valor` produz um valor do tipo `par`. Não encadeia: `a -> b -> c` é erro.  
**Raiz quadrada:** `importe "mat"` e então `raiz(n)` — erro se `n < 0` (tratável com `se_falhar`)  
**Ao acaso:** no mesmo módulo, `aleatorio(min, max)` (também `aleatório`) inteiro inclusive; `semente(n)` fixa a sequência.  
**Trigonometria e análise:** no mesmo módulo, ângulos em **radianos**. `seno`/`sen`, `cosseno`/`cos`, `tangente`/`tan`; inversas `arcoseno`, `arcocosseno`, `arcotangente`, `arcotangente2(y, x)`. `radianos(g)` e `graus(r)` convertem. `pi` é um valor (não `pi()`). Também `exp`, `log` (natural), `log10`, `potencia`, `piso`, `teto`, `arredonde`. Domínio inválido (por exemplo `log(0)`, `arcoseno(2)`) é erro capturável com `se_falhar`. `e` é palavra-chave; use `exp(1)`.  
**Texto ↔ número:** `numero(t)` (também `número`) lê o padrão atual; `formato("pt")` / `formato("en")` escolhe pt-BR (`1.000,5`) ou en-US (`1,000.5`). Padrão pt-BR. Literais no código usam `.`.  
**Aritmética:** padrão decimal (`formato("decimal")`). `formato("float")` liga IEEE-754 só nas operações; o padrão de texto (`pt`/`en`) continua independente. Variável de ambiente `EXPRESSA_ARITMETICA`; CLI `--aritmetica decimal|float`.  
**Acesso:** `mat::soma` (módulo), `pessoa:nome` (chave de mapa), `p:chave` / `p:valor` (par), `xs.tamanho()` (= `tamanho(xs)`).

```text
10 + 5
x > 10 e x < 20
nao verdadeiro
```

---

## 5. Blocos (tudo é expressão)

```text
resultado = inicio
    10 + 5
fim
// resultado = 15

resultado = { 10 + 5 }    // igual
```

---

## 5.1. Matrizes

Retangulares, só números. Vêm do módulo `matriz`. Índices começam em 1: `A[linha, coluna]`. `A[i]` devolve a linha como lista.

```text
importe "matriz"

A = matriz([
    [1, 2, 3],
    [4, 5, 6],
])

A[1, 2]          // 2
A + B            // mesma ordem
3 * A
A * B            // produto de matrizes
transposta(A)    // ou A.transposta()
det(A)           // 1×1, 2×2 ou 3×3
identidade(3)
zeros(2, 5)      // só zeros; uns(2, 5); cheia(2, 5, 7)
nlinhas(A)       // 2
ncolunas(A)      // 3
tamanho(A)       // também linhas
```

O construtor é a função `matriz(linhas)`: uma lista de listas de números, retângulo obrigatório. Depois de `importe "matriz"`, o nome `matriz` é essa função; `A.matriz::transposta()` só vale com alias (`matriz = importe "matriz"`, então `matriz::matriz([[…]])` e `A.matriz::transposta()`).

---

## 6. Condicionais

Cada ramo tem seu próprio bloco (`inicio`/`fim` ou `{`/`}`).

Como **comando**, `senao` é opcional. Como **valor** (`x = se …`, argumento, `r = { se … }`), `senao` é obrigatório. No corpo da função, um `se` no fim continua sendo comando.

```text
se nota >= 7
inicio
    "aprovado"
fim
ou se nota >= 5
inicio
    "recuperação"
fim
senao
inicio
    "reprovado"
fim
```

---

## 7. Laços

```text
// Repetir N vezes
repita 3 vezes
inicio
    escreva("olá")
fim

// Contador
para i de 1 ate 5
inicio
    escreva(i)
fim

// Percorrer lista
para nome em ["Ana", "Bruno"]
inicio
    escreva(nome)
fim

// Enquanto a condição for verdadeira
i = 1
enquanto i <= 3
inicio
    escreva(i)
    i = i + 1
fim
```

A condição de `enquanto` tem que ser `verdadeiro` ou `falso`. O corpo usa `inicio`/`fim` ou `{`/`}`.

`para x em` percorre **lista** ou **texto** (um caractere por vez):

```text
para ch em "olá"
inicio
    escreva(ch)        // o  l  á
fim
```

`pare` sai do laço mais interno (`para`, `enquanto`, `repita`). `continue` (ou `continua`) pula para a próxima volta. Fora de um laço é erro. Não são capturados por `se_falhar`.

---

## 8. Funções (primeira classe)

```text
soma = funcao(x, y)
inicio
    x + y                  // última expressão = resultado
fim

escreva(soma(10, 5))     // 15

busca = funcao(xs, alvo)
inicio
    para x em xs
    inicio
        se x == alvo
        inicio
            retorne verdadeiro     // sai da função, não só do se
        fim
    fim
    falso
fim
```

`retorne` só vale **dentro de uma função**. `retorne` sozinho devolve `nada`. O valor, se houver, fica **na mesma linha**. Não é capturado por `se_falhar`.

`se` usado **como valor** (`x = se …`, argumento, última expressão da função) precisa de `senao`. Como comando, o `senao` é opcional:

```text
se n < 0 { escreva("negativo") }           // ok
x = se n >= 7 { "ok" }                     // erro: falta senao
x = se n >= 7 { "ok" } senao { "não" }     // ok
```

---

## 9. Listas

- Indexação começa em **1**
- Criação: `[1, 2, 3]`

```text
numeros = [10, 20, 30, 40]

tamanho(numeros)               // 4
numeros[1]                     // 10
numeros + [50]                 // [10, 20, 30, 40, 50]
numeros += [50]                // igual a numeros = numeros + [50]
numeros.remova(2)              // sem o 2º item (cópia)
numeros.remova(2, 3)           // sem os índices 2 e 3
numeros contem 20              // verdadeiro
primeiro(numeros)              // 10
ultimo(numeros)                // 40
numeros[2..3]                  // [20, 30]
numeros[2..]                   // do 2 até o fim
"b"[1..3]                      // "b" (corta no tamanho)
```

Índices de **texto** também começam em **1** e falam de caracteres Unicode (não bytes). `t[2]` lê um caractere. `t[2] = "b"` troca esse caractere e reatribui o texto; o valor precisa ser um texto de tamanho 1. O tamanho do texto permanece o mesmo. `a = t` depois `t[1] = "x"` deixa `a` com o texto antigo.

```text
s = "ABCDE"
s[2] = "b"                     // "AbCDE"
linhas[1][2] = "x"             // se linhas[1] é texto, reescreve o caractere
```

---

## 10. Pares e mapas

`chave -> valor` é um **par**. Campos: `p:chave` e `p:valor`. Chaves de par são texto, número inteiro ou bool (`:nome` é o texto `"nome"`). Palavras-chave e textos com espaço continuam com aspas. Um par **não** serve de chave de mapa nem de elemento de conjunto.

`mapa` e `conjunto` são funções do núcleo. `mapa()` (ou `mapa([])`) é o mapa vazio. `mapa(xs)` recebe **uma** lista de pares; chaves duplicadas são erro.

```text
devs = [:developer -> "Thiago", "co-developer" -> "Grok"]
pessoa = mapa([
    :nome -> "Thiago",
    :idade -> 25,
    "cidade natal" -> "Fortaleza",
])

pessoa:nome                    // "Thiago"
pessoa[:nome]                  // igual a pessoa["nome"]
pessoa:nome = "Bia"
pessoa contem :idade           // verdadeiro
pessoa = pessoa.remova(:idade)
```

`pessoa:nome` exige os tokens **na mesma linha**. Com quebra de linha, `:nome` é o texto `"nome"` (chave), não um campo:

```text
pessoa:nome          // campo
opções
:args                // não é opções:args
```

Campo como valor de um par: `:op -> (obj:campo)`. `{ }` e `inicio`/`fim` são só blocos; `[ ]` é só lista (vírgulas obrigatórias, vírgula final permitida).

---

## 11. Conjuntos

Valores únicos (texto, número inteiro ou bool). Sem repetir, sem ordem de índice. `conjunto()` (ou `conjunto([])`) é vazio. `conjunto(xs)` recebe uma lista e descarta duplicatas na ordem de inserção.

```text
s = conjunto([:ana, :bia, 1])
s contem :ana              // verdadeiro
s += :carlos               // acrescenta um elemento
s += conjunto([:bia, :dani])    // união
s = s.remova(:ana)         // por valor, devolve cópia
tamanho(s)
para x em s { escreva(x) }
```

Lista e mapa **não** viram conjunto sozinhos. `remova` no conjunto é o elemento, não um índice.

---

## 12. Textos (strings)

```text
nome = "  Maria Silva  "
nome[1]                    // um caractere
nome[2..4]                 // inclusive; se o fim passar do tamanho, corta
nome[2..]                  // até o último

tamanho(nome)
maiuscula(nome)
sem_acento("olá")          // "ola"
minuscula(nome)
nome contem "Silva"
procurar(nome, "Silva")    // índice 1…n; 0 se não achar
substitua(nome, "Maria", "Ana")
separe("a,b,c", ",")
junte(["a", "b"], " - ")
limpe(nome)
formate("{:<8} {:>5}", "Ana", 10)   // < esquerda  > direita  ^ centro
nome[1]
nome[1..5]
```

---

## 13. Entrada do teclado

`leia()` lê uma linha do teclado (sem a quebra de linha).  
`leia(prompt)` imprime o texto do prompt e em seguida espera a linha.

```text
nome = leia("Qual o seu nome? ")
escreva("Olá, " + nome)
```

Fim da entrada (Ctrl+D / Ctrl+Z) é um erro, tratável com `se_falhar`.

`leia_linhas()` lê **todas** as linhas restantes da entrada padrão e devolve uma lista (arquivo vazio → `[]`). Não é erro chegar ao fim.

```text
busca = argumentos[1]
para linha em leia_linhas()
inicio
    se linha contem busca
    inicio
        escreva(linha)
    fim
fim
```

```text
cat nomes.txt | expressa grep.lep Thiago
```

`eh_terminal()` (também `é_terminal()`), no módulo `tela`, é `verdadeiro` se a entrada é o teclado, `falso` se é um pipe ou um arquivo.

`argumentos` é a lista dos valores depois do `.lep` na linha de comando (`argumentos[1]` é o primeiro; índices começam em 1). Sem argumentos extras, a lista é `[]`. Não dá para reatribuir `argumentos`.

`avaliar(texto)` executa o texto como código Expressa no escopo atual e devolve o último valor. `avaliar("2 + 3 * 4")` vale `14`. Erros de sintaxe ou de execução (por exemplo divisão por zero) são capturáveis com `se_falhar`.

`catalogo()` (também `catálogo()`) devolve uma lista de mapas com as funções e módulos visíveis: `:nome`, `:tipo` (`nativa`, `funcao`, `modulo`), `:args`, `:modulo`. `catalogo(alvo)` inspeciona um mapa, um módulo, uma função ou o nome de um módulo nativo (`catalogo("matriz")`). No REPL, `ls` formata o catálogo; `ls matriz` e `ls c` inspecionam o alvo. Uma `funcao` impressa mostra os parâmetros: `<funcao(x)>`.

`escreva` vai para a saída padrão (stdout). `escreva_erro` vai para a saída de erro (stderr).  
Com `importe "tela"`: `cls()` (também `limpe_tela()`) limpa a tela; `casa()` manda o cursor ao canto sem apagar — use no lugar de `cls()` em animações para não piscar.  
`colunas()` e `linhas()` são o tamanho da tela em caracteres (80×24 se não houver TTY). `nova_linha` é o texto `"\n"` no Unix e `"\r\n"` no Windows (valor, não função). `quadro(texto)` volta ao canto, escreve o texto, apaga o resto de cada linha e o que sobrar embaixo; as quebras viram `nova_linha` na saída. `bloco(linha, coluna)` devolve o mapa `{:linha, :coluna}` (1-based); `escreva_em(bloco, texto)` (UFCS `b.escreva_em("…")`) escreve uma linha nessa coluna, anda `:linha` e devolve o mesmo mapa. Sem `"\n"` no texto.  
`pinte(texto, frente)` e `pinte(texto, frente, fundo)` devolvem o texto com cor ANSI e reset no fim. `fundo(texto, cor)` pinta só o papel; `negrito(texto)` deixa em negrito. Cores: `normal`, `preto`, `vermelho`, `verde`, `amarelo`, `azul`, `magenta`, `ciano`, `branco`.  
`durma(segundos)` espera; `durma(0.5)` é meio segundo.

`sair()` encerra o programa com código 0. `sair(1)` encerra com falha (o shell vê o código). Não é capturado por `se_falhar`.

```text
se nome == ""
inicio
    escreva_erro("nome vazio")
    sair(1)
fim
```

```text
escreva("ok")
escreva_erro("falhou")
```

---

## 14. Arquivos

Funções do módulo `arquivo`:

```text
importe "arquivo"
linhas = leia_arquivo("dados.txt")          // retorna lista de linhas
salve_arquivo("saida.txt", linhas)          // salva lista
adicione_arquivo("saida.txt", ["nova"])     // adiciona linhas
```

**CSV:**
```text
dados = leia_csv("pessoas.csv")             // lista de listas
salve_csv("saida.csv", dados)
```

---

## 15. Tratamento de Erros

- Qualquer erro causa **crash** (com valores + call stack)
- Para tratar, usa-se `se_falhar`

```text
importe "arquivo"
linhas = leia_arquivo("arquivo.txt") se_falhar []

valor = 10 / 0 se_falhar 0

item = lista[99] se_falhar "não existe"
```

---

## 16. Módulos / Importação

A mesma forma vale para arquivos `.lep` e para os módulos nativos `matriz`, `arquivo`, `tela` e `mat`.

```text
mat = importe "matematica"     // alias: nomes em mat::
mat::soma(10, 5)

importe "matematica"           // espalha os nomes no escopo atual
soma(10, 5)
```

Nome **sem** `/`, `./` nem `.lep` procura primeiro um módulo nativo. Caminho com `./`, `/` ou sufixo `.lep` é arquivo. Se o nome nativo também existir como arquivo no mesmo diretório (`matriz.lep` ao lado do programa), `importe "matriz"` é **erro**: use `importe "./matriz"` para o arquivo. Enquanto o arquivo estiver no caminho do nome nu, o nativo fica inacessível por esse nome.

Depois de espalhar `importe "matriz"`, o construtor `matriz` ocupa o identificador; use `A.transposta()`. Com alias `matriz = importe "matriz"`, use `matriz::matriz([[…]])` e `A.matriz::transposta()`.

---

## 17. Execução do Programa

O código executa **de cima para baixo**, linha por linha.  
Não existe função `main` obrigatória.

Arquivos `.lep` podem ser executáveis:

```text
#!/usr/bin/env expressa
escreva("olá")
```

```text
chmod +x ola.lep
./ola.lep
```

Valores depois do script chegam em `argumentos` (`./ola.lep Thiago` → `argumentos[1]` é `"Thiago"`).

---

## Exemplo Completo

```text
// Calcula a média de uma lista de notas

notas = [7.5, 8.0, 6.5, 9.0, 5.5]

soma = 0
para nota em notas
inicio
    soma = soma + nota
fim

media = soma / tamanho(notas)

resultado = se media >= 7
inicio
    "Aprovado"
fim
senao
inicio
    "Reprovado"
fim

escreva("Média: " + media)
escreva("Resultado: " + resultado)
```
