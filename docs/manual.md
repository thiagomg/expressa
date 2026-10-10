# Manual Expressa

Documentação para quem **usa** a linguagem. Exemplos curtos, uma função de cada vez.

| Texto | Papel |
|-------|--------|
| [Tutorial](../tutorial/basico.md) | sair do zero |
| [Tutorial avançado](../tutorial/avancado.md) | o resto, com programas |
| **Este manual** | consulta: o que cada coisa faz |
| [Folha de consulta](../folha-de-consulta.md) | uma página |
| [Especificação](../especificacao-linguagem.md) | regras finas (quem implementa a linguagem) |

Na Aula, **F1** (ou `ajuda("escreva")`) mostra a mesma ficha que está em `docs/nativas/`. O REPL: `ajuda escreva`.

Arquivos de programa: `.lep`. Índices começam em **1**.

---

## Executar

```text
expressa                 REPL
expressa prog.lep        executa o arquivo
expressa prog.lep Ana    argumentos[1] é "Ana"
expressa debug prog.lep  depurador
```

Na Aula: **Executar** (F5). `leia` responde na saída. Caixa **argumentos** = `argumentos[1]`….

Shebang: `#!/usr/bin/env expressa` na primeira linha, `chmod +x prog.lep`.

---

## Tipos

| Tipo | Exemplo | Notas |
|------|---------|--------|
| número | `10` `3.14` `-8` | no código, ponto; valor decimal exato (`0.1+0.2` é `0.3`) |
| texto | `"olá"` | |
| bool | `verdadeiro` `falso` | |
| lista | `[1, 2, 3]` | `xs[1]` é o primeiro |
| par | `:nome -> "Ana"` | `p:chave` `p:valor` |
| mapa | `mapa([:nome -> "Ana"])` | `pessoa:nome` |
| conjunto | `conjunto([:ana, 1])` | valores únicos |
| matriz | `matriz([[1, 2], [3, 4]])` | só números; `importe "matriz"` |
| função | `função(x) { x + 1 }` | também `funcao` |
| módulo | `importe "mat"` | |
| nada | — | sem literal; `escreva` devolve nada |

`:nome` é o texto `"nome"`.

---

## Nomes, blocos, comentários

```text
x = 10                    // cria ou altera
inicio                    // ou { … }
    y = 20                // some no fim do bloco
fim
resultado = { 10 + 5 }    // bloco como valor → 15
```

`inicio` fecha com `fim`; `{` fecha com `}`. Não misture no mesmo bloco.

```text
// uma linha
/* várias linhas */
```

`#` só vale na primeira linha do arquivo (`#!`). Funções **leem** nomes de fora e **não alteram**.

---

## Operadores

```text
+  -  *  /  %             // + cola texto se um lado é texto
==  !=  >  <  >=  <=
e  ou  nao
xs contem 20
"Maria Silva" contem "Silva"
mapa contem "idade"
:nome -> "Ana"            // par
pessoa:nome               // chave de mapa
xs.tamanho()              // igual a tamanho(xs); () obrigatório
mat::soma(1, 2)           // nome no módulo
i += 1                    // números: += e -=
nome += " Silva"          // texto: só +=
```

Sem `a < b < c`: use `a < b e b < c`.

---

## Controle

```text
se nota >= 7 { "aprovado" } senão { "reprovado" }
se n < 0 { escreva("negativo") }     // senão opcional neste caso
ou se nota == 10 { escreva("gabaritou") }

repita 3 vezes { escreva("olá") }
para i de 1 até 5 { escreva(i) }
para nome em ["Ana", "Bruno"] { escreva(nome) }
para ch em "olá" { escreva(ch) }

i = 1
enquanto i <= 3 {
    escreva(i)
    i += 1
}
pare          // sai do laço
continue      // próxima volta
```

Como **valor**, o `se` precisa de `senão`. Como **comando**, `senão` é opcional.

`retorne x` só dentro de função; sai na hora. `retorne` sozinho devolve nada.

---

## Funções

Última expressão do bloco é o resultado.

```text
soma = função(x, y) { x + y }
escreva(soma(10, 5))      // 15

busca = função(xs, alvo) {
    para x em xs {
        se x == alvo { retorne verdadeiro }
    }
    falso
}
```

`função` e `funcao` são o mesmo. O primeiro argumento pode ir na frente: `xs.tamanho()`.

---

## Módulos

```text
mat = importe "matematica"
mat::soma(10, 5)

importe "matematica"      // nomes soltos
soma(10, 5)
```

Nativos: `tela`, `mat`, `matriz`, `arquivo`. Nome sem `/` nem `.lep` procura o nativo primeiro. `importe "./matriz"` força arquivo local.

---

## Erros

Sem `se_falhar`, o erro encerra o programa. Com, vale o da direita.

```text
valor = 10 / 0 se_falhar 0
linhas = leia_arquivo("x.txt") se_falhar []
```

`sair(1)` encerra de propósito e **não** é capturado por `se_falhar`.

---

## Núcleo

Sempre visível, sem `importe`.

### `escreva(valor, …)` / `escreva()`

Imprime os argumentos separados por espaço e quebra a linha. Sem argumentos, linha vazia. Devolve nada.

```text
escreva("Olá", 10)
escreva()
```

### `escreva_erro(valor, …)`

Como `escreva`, na saída de erro (stderr). Não entra em `> arquivo`.

```text
escreva_erro("nome vazio")
```

### `sair()` / `sair(codigo)`

Encerra. Sem argumento, código 0. Nada depois da chamada executa. No REPL, a linha `sair` (sem `()`) sai do interpretador; `sair()` encerra o processo.

```text
se nome == "" { sair(1) }
```

### `durma(segundos)`

Espera. Aceita fração (`durma(0.5)`). `durma(0)` não espera.

```text
durma(1)
```

### `leia()` / `leia(prompt)`

Uma linha do teclado, sem o Enter. Prompt opcional, sem quebra no fim. Fim da entrada é erro (`se_falhar`).

```text
nome = leia("Seu nome: ")
```

### `leia_linhas()`

Resto da entrada padrão até o fim → lista de textos. Vazio = `[]`. Na Aula não há EOF: a chamada é erro.

```text
para linha em leia_linhas() { escreva(linha) }
```

### `argumentos`

Lista, não função. O que veio depois do `.lep`. `argumentos[1]` é o primeiro. No REPL: `[]`. Não dá para fazer `argumentos = …`.

```text
escreva(argumentos[1])
```

### `numero(texto)` / `número(texto)`

Texto → número. Aceita `3.14` ou `3,14` conforme `formato`. Espaços nas pontas e `_`. Se já for número, devolve o mesmo.

```text
idade = numero(leia("Idade: ")) se_falhar 0
```

### `formato(padrao)`

Dois eixos, independentes:

- texto: `"pt"` (`1.000,5`, padrão) ou `"en"` (`1,000.5`)
- aritmética: `"decimal"` (padrão, `0.1+0.2` é `0.3`) ou `"float"` (IEEE-754)

Também `"exato"`, `"flutuante"`. CLI: `--numeros en`, `--aritmetica float`. Literais no código continuam com ponto (`7.3`).

```text
formato("en")
formato("float")
```

### `formate(modelo, valor, …)`

`{}` vira o valor. `{:<n}` esquerda, `{:>n}` direita, `{:^n}` centro. `{1}` é o primeiro valor. `{{` `}}` escrevem `{` `}`.

```text
formate("{:<8} {:>5}", "Ana", 10)
```

### `avaliar(texto)`

executa o texto como Expressa no escopo atual e devolve o último valor. Erro de sintaxe ou de execução vai em `se_falhar`.

```text
avaliar("2 + 3 * 4")     // 14
```

### `catalogo()` / `catalogo(alvo)` / `catálogo`

Sem argumento: o que está visível. Com mapa, módulo, `"matriz"` ou uma função: os itens (`:nome`, `:tipo`, `:args`, `:modulo`, `:doc`).

```text
catalogo()
catalogo("matriz")
```

### `ajuda(alvo)`

Ficha de nativa ou função: `"escreva"`, `"matriz::zeros"`, ou a própria função.

```text
ajuda("escreva")
```

### `mapa()` / `mapa(pares)`

Mapa vazio ou a partir de pares. Chaves duplicadas são erro.

```text
pessoa = mapa([:nome -> "Ana", :idade -> 25])
pessoa:nome = "Bia"
pessoa["cidade"] = "Fortaleza"
pessoa = pessoa.remova("idade")
```

### `conjunto()` / `conjunto(xs)`

Valores únicos (texto, inteiro ou bool). Duplicata some; a ordem de inserção fica.

```text
s = conjunto([:ana, :bia])
s += :carlos
s = s.remova(:bia)
```

### `tamanho(valor)`

Caracteres, itens, pares ou linhas de matriz. Texto conta Unicode, não bytes.

```text
tamanho("olá")    // 3
```

### `primeiro(lista)` / `ultimo(lista)`

Índice 1 e último. Erro se a lista está vazia.

```text
primeiro([10, 20])
```

### `maiuscula(texto)` / `minuscula(texto)`

Cópia em maiúsculas / minúsculas.

```text
maiuscula("olá")
```

### `sem_acento(texto)`

Tira acento e cedilha (`ã`/`á`→`a`, `ç`→`c`). Não muda maiúscula/minúscula.

```text
sem_acento("São Paulo")
```

### `remova(lista, i)` / `remova(lista, a, b)` / `remova(mapa, chave)`

Cópia sem o índice (faixa inclusiva) ou sem a chave. No conjunto, some o elemento.

```text
xs = xs.remova(2)
```

### `substitua(texto, antigo, novo)`

Troca todas as ocorrências.

```text
substitua("aa", "a", "b")    // "bb"
```

### `procurar(texto, trecho)` / `procurar(lista, item)`

Índice 1…n da primeira ocorrência; `0` se não achar. Trecho `""` é erro.

```text
procurar("onde", "n")    // 2
```

### `separe(texto, separador)`

Parte o texto. Separador `""` → um item por caractere.

```text
separe("a,b", ",")    // ["a", "b"]
```

### `junte(lista, separador)`

Cola os itens. Cada um vira texto.

```text
junte(["a", "b"], "-")    // "a-b"
```

### `limpe(texto)`

Tira espaços do começo e do fim.

```text
limpe("  x  ")    // "x"
```

### Listas e fatias

```text
xs = [10, 20, 30, 40]
xs[1]           // 10
xs[2..3]        // [20, 30]
xs[2..]         // até o fim
xs + [50]
xs += [50]
"abc"[1..2]     // "ab"
t[2] = "x"      // um caractere; reatribui t
```

Nativas do núcleo não podem ser reatribuídas (`escreva = 1` é erro).

---

## `importe "tela"`

Terminal e painel da Aula.

### `cls()` / `limpe_tela()`

Limpa e põe o cursor no canto. No REPL, a linha `cls` também limpa. Em animação, `cls` a cada quadro pisca: use `casa()`.

### `casa()`

Cursor no canto, sem apagar. Na Aula, o painel trata como novo quadro.

### `eh_terminal()` / `é_terminal()`

Verdadeiro se a entrada é o teclado. Falso em pipe, arquivo e Aula.

### `pinte(texto, frente)` / `pinte(texto, frente, fundo)`

Devolve o texto com cor ANSI e reset no fim. Cores: `normal preto vermelho verde amarelo azul magenta ciano branco`. `:verde` é o texto `"verde"`. `normal` é a cor padrão daquele slot.

```text
"HP".pinte(:verde)
"HP".pinte(:branco, :vermelho)
```

### `fundo(texto, cor)`

Só o papel; letra no padrão do terminal.

### `negrito(texto)`

Negrito ANSI e reset no fim.

### `colunas()` / `linhas()`

Tamanho em caracteres. Sem TTY: 80×24. Na Aula hoje também 80×24.

### `quadro(texto)`

Cursor no canto, escreve o texto, apaga o resto de cada linha e o que estava embaixo. Monte as linhas com `nova_linha`.

### `nova_linha`

Valor, não função: `\n` no Unix e na Aula; `\r\n` no Windows.

```text
tela += linha + nova_linha
```

### `bloco(linha, coluna)`

Mapa `{:linha, :coluna}` (1-based). Não desenha.

### `escreva_em(bloco, texto)`

Uma linha na coluna do bloco; devolve o mesmo mapa (a linha anda). UFCS: `b.escreva_em("Q W E")`. Sem `\n` no texto. O que estava à esquerda na mesma linha permanece.

```text
b = bloco(1, 28)
b.escreva_em("Q W E R T")
```

---

## `importe "mat"`

### `raiz(n)`

Raiz quadrada. Negativo é erro (`se_falhar`).

### `aleatorio(min, max)` / `aleatório`

Inteiro inclusive. Sem `semente()`, cada execução muda.

### `semente(n)`

Fixa a sequência de `aleatorio`.

### `pi`

Valor, não função: `pi`, não `pi()`.

```text
area = pi * r * r
```

### `seno` / `sen` · `cosseno` / `cos` · `tangente` / `tan`

Argumento em **radianos**. Graus: `seno(radianos(30))`.

### `arcoseno` `arcocosseno` `arcotangente` `arcotangente2(y, x)`

Inversas. `arcoseno`/`arcocosseno`: argumento entre -1 e 1.

### `radianos(graus)` / `graus(radianos)`

Conversão. `graus(pi)` é 180.

### `exp(x)` · `log(x)` · `log10(x)`

`exp` é e^x. `log` é natural. `log10(1000)` é 3. `log`/`log10` pedem x > 0.

### `potencia(base, exp)`

Expoente pode ser fração. `0^0` vale 1.

### `piso(x)` · `teto(x)` · `arredonde(x)`

Chão, teto, inteiro mais próximo. Empate (`n,5`) afasta de zero: `arredonde(1.5)` é 2, `arredonde(-1.5)` é -2.

---

## `importe "matriz"`

Só números. `A[1, 2]` é linha 1, coluna 2. `A[1]` é a linha como lista.

```text
A = matriz([[1, 2, 3], [4, 5, 6]])
A[1, 2]          // 2
transposta(A)
det(A)           // 1×1, 2×2 ou 3×3
identidade(3)
zeros(2, 5)
uns(3)
cheia(2, 3, 7)
nlinhas(A)       // 2
ncolunas(A)      // 3
tamanho(A)       // também as linhas
A + B
3 * A
A * B            // produto de matrizes
```

---

## `importe "arquivo"`

Caminhos relativos: pasta do `.lep`, não a pasta de onde você chamou `expressa`. Na Aula, só a pasta do projeto.

### `leia_arquivo(caminho)`

Lista de textos (uma linha cada, sem a quebra). Vazio → `[]`. Falha de leitura é erro (`se_falhar`).

```text
linhas = leia_arquivo("dados.txt") se_falhar []
```

### `salve_arquivo(caminho, linhas)`

Substitui o arquivo. Cria a pasta se faltar. Cada item vira uma linha.

### `adicione_arquivo(caminho, linhas)`

Acrescenta no fim (cria se não existir).

### `leia_csv(caminho)` / `salve_csv(caminho, dados)`

CSV simples (vírgula, sem aspas). Lista de listas de **texto**. `salve_csv` substitui o arquivo.

```text
dados = leia_csv("pessoas.csv")
salve_csv("saida.csv", [["Ana", 25]])
```

---

## REPL e depurador

```text
expressa
ajuda
ajuda funcoes
ajuda linguagem
ajuda escreva
ajuda matriz::zeros
ls              // no REPL, como catalogo
cls             // limpa a tela do REPL
sair
```

```text
expressa debug prog.lep
continuar (c)   proximo (n)   entrar (s)   sair (o)
vars (v)   pilha (k)   ponto N   terminar (q)
```

Na Aula: **F6** depura; F9 ponto de parada; F10 próximo; F11 entrar.

---

## Números: texto e aritmética

Literais no código: sempre ponto (`3.14`).

`formato("pt")` / `formato("en")` mudam só a **escrita** e o `numero("3,14")`.

`formato("decimal")` (padrão) soma em base 10. `formato("float")` usa IEEE-754 em `+ - * / %`. Independente de pt/en.

`mat` e `matriz` calculam em binário por dentro (seno, determinante).
