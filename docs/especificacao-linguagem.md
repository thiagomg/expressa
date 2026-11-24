<!--
[ID]: # (169813dc-920f-46c3-a229-92ad27cc32a8)
[DATE]: # (2026-08-10 02:27:16.000)
[AUTHOR]: # (Thiago Massari Guedes)
[TAGS]: # ()
-->
# Expressa: especificação da linguagem

A Expressa é uma linguagem de programação voltada a iniciantes, com palavras-chave em **português brasileiro**. Esta especificação descreve sua sintaxe e suas regras de execução.

Para aprender a linguagem, consulte os tutoriais [básico](tutoriais/basico.md) e [avançado](tutoriais/avancado.md). Para consultar funções e exemplos de uso, veja o [manual](manual.md).

Arquivos de código-fonte usam a extensão `.lep`; arquivos compilados em bytecode usam `.lepc`.

---

Um bloco pode ser delimitado por `inicio` e `fim` ou por `{` e `}`. As duas formas são equivalentes, mas os delimitadores devem corresponder: `inicio` termina com `fim`, e `{` termina com `}`. Blocos vazios não são aceitos.

Blocos podem ser usados como expressões. O resultado de um bloco é o valor da última expressão executada nele. Atribuições e laços não substituem esse valor; se nenhuma expressão for executada, o resultado é `nada`. Atribuições, laços e importações não podem ocupar qualquer posição em que se espera uma expressão.

Uma expressão sem efeitos, cujo valor é descartado antes da última linha do bloco, produz o erro `este valor é calculado e não é usado`. Chamadas de função, blocos e condicionais `se` podem aparecer nessas linhas. O corpo de um laço descarta todos os valores, inclusive o da última linha. A última linha do arquivo ou de uma entrada do REPL pode conter uma expressão cujo valor não seja atribuído a uma variável nem passado a uma função.

---

## 1. Tipos básicos

| Tipo     | Exemplos                          |
|----------|-----------------------------------|
| `numero` | `10`, `3.14`, `-8`, `0`, `1_000` |
| `texto`  | `"olá"`, `"123"`                  |
| `bool`   | `verdadeiro`, `falso`             |
| `nada`   | `nada` (ausência de valor)        |
| `intervalo`  | `0..3`, `1..=2`, `5..`            |

`nada` é um literal reservado de ausência de valor. Pode ser atribuído a variáveis, comparado com `==`/`!=`, usado em coleções e retornado explicitamente. Não pode ser usado como identificador.

Além desses tipos, há **lista**, **par**, **mapa**, **conjunto**, **matriz**, **imagem**, **função**, **módulo** e **processo**.

Os contextos que exigem números inteiros, como chaves numéricas, elementos numéricos de conjuntos, contadores e índices, usam inteiros com sinal de 64 bits: de −9223372036854775808 a 9223372036854775807. O tipo `numero` aceita valores fora desse intervalo; um número sem parte fracionária ainda pode ultrapassar os limites desses contextos. Dimensões e índices também estão sujeitos à capacidade de endereçamento da plataforma e aos limites definidos pelo ambiente de execução. Conversões fora do intervalo permitido produzem erro; o valor não é substituído por zero.

O limite final inclusivo de `..=` deve ser menor que o maior inteiro de 64 bits, pois a representação interna usa o sucessor desse limite. Caso contrário, a mensagem é `o teto de ..= passa do maior inteiro`, com referência ao limite informado no programa.

O caractere `_` pode separar dígitos em um literal numérico (`1_000`, `3.14_15`). Não pode aparecer no fim do literal, repetido ou ao lado do ponto decimal. Não há notação científica: `1e5` é inválido. Mensagens de erro apresentam números no formato ativo (`2,5` em pt-BR); arquivos CSV e argumentos enviados a programas externos usam ponto como separador decimal.

`tipo(valor)` devolve o nome do tipo como texto (`"numero"`, `"intervalo"`, `"texto"`, `"bool"`, `"lista"`, `"par"`, `"mapa"`, `"conjunto"`, `"matriz"`, `"imagem"`, `"funcao"`, `"modulo"`, `"processo"`, `"nada"`). Os nomes são os mesmos das mensagens de erro (sem acento). `escreva` e funções do aluno são `"funcao"`. `tipos()` lista esses nomes.

Toda função nativa cujo nome tem uma palavra acentuada em português aceita as duas grafias: `maiuscula`/`maiúscula`, `eh_pasta`/`é_pasta`, `laco_grafico`/`laço_gráfico`.

---

## 2. Variáveis e escopo

- Declaração: apenas `nome = valor`
- Reatribuição permitida
- Escopo de bloco: variáveis criadas dentro de `inicio...fim` (ou `{...}`) só existem dentro dele
- Funções podem **ler** nomes de fora, mas **não podem reatribuir esses nomes**. Listas, mapas e matrizes são objetos compartilhados (§9.1): alterar um elemento desses objetos é permitido, inclusive quando o nome está em um escopo externo.

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

Um comentário iniciado por `///` documenta a definição seguinte (`nome = …` ou `importe`). Várias linhas consecutivas com `///` formam uma única documentação, consultável com `ajuda(soma)`. A última delas deve estar imediatamente acima da definição: uma linha em branco ou um comentário `//` entre elas produz o erro `documentação só vale antes de uma definição`. `////` e `///` no meio de uma linha são comentários comuns.

A **primeira linha** do arquivo pode ser um shebang Unix (`#!…`). A Expressa ignora essa linha. `#` **não** é comentário em qualquer outro lugar.

As funções nativas estão documentadas em `docs/nativas/*.lep`, com o mesmo formato `///`. Essa documentação pode ser consultada com chamadas como `ajuda("escreva")` e `ajuda("matriz::zeros")`.

```text
#!/usr/bin/env expressa
```

---

## 4. Operadores

**Aritméticos:** `+`, `-`, `*`, `/` e `%`. O operador `%` calcula o resto da divisão cujo quociente é truncado em direção a zero: `-7 % 3` é `-1`.

**Atribuição composta:** `+=` e `-=` aceitam os mesmos operandos de `+` e `-`, respectivamente. `alvo += v` lê o valor do alvo, soma `v` e grava o resultado no mesmo destino. O objeto acessado e seus índices são avaliados uma única vez, na ordem em que aparecem, antes de `v`. Na atribuição simples, o lado direito é avaliado primeiro; em um acesso `[i]`, o índice é avaliado antes do objeto acessado. Ao substituir um caractere de um texto, a gravação também usa o destino determinado nessa avaliação.

**Comparação:** `==`, `!=`, `>`, `<`, `>=` e `<=`. Textos são comparados pela ordem dos pontos de código Unicode; por isso, `"ábaco" > "zebra"` é verdadeiro. O operador `nao` tem precedência maior que `==`. Aplicá-lo diretamente a um número produz um erro com a sugestão de escrever uma condição como `nao (x == 3)`.

**Lógicos:** `e` `ou` `nao`  
**Par:** `chave -> valor` produz um valor do tipo `par`. Esse operador não pode ser encadeado: `a -> b -> c` é inválido.

**Raiz quadrada:** `importe "mat"` e então `raiz(n)` — erro se `n < 0` (tratável com `se_falhar`)  
**Números aleatórios:** no módulo `mat`, `aleatorio(min, max)` (também `aleatório`) devolve um inteiro entre `min` e `max`, incluindo os dois limites. `semente(n)` define a semente usada para gerar a sequência.

**Trigonometria e outras funções matemáticas:** o módulo `mat` usa ângulos em **radianos**. Oferece `seno`/`sen`, `cosseno`/`cos` e `tangente`, além das funções inversas `arcosseno`, `arcocosseno`, `arcotangente` e `arcotangente2(y, x)`. `radianos(g)` converte graus em radianos; `graus(r)` faz a conversão inversa. `pi` é um valor, usado sem parênteses.

O módulo também oferece `exp`, `log` (logaritmo natural), `log10`, `potencia` (também `potência`), `piso`, `teto` e `arredonde`. As três funções de arredondamento operam diretamente sobre o valor decimal. `arredonde` escolhe o inteiro mais próximo; em caso de empate, escolhe o de maior valor absoluto (`2.5` → `3`, `-2.5` → `-3`). Argumentos fora do domínio da função, como `log(0)` e `arcosseno(2)`, produzem erros tratáveis com `se_falhar`. Como `e` é palavra-chave, use `exp(1)` para obter a constante de Euler.

**Conversão entre texto e número:** `numero(t)` (também `número`) interpreta o texto conforme o formato numérico ativo. `define_opcao(:formato_numero, :pt)` seleciona pt-BR (`1.000,5`); `define_opcao(:formato_numero, :en)` seleciona en-US (`1,000.5`). O padrão é pt-BR. Nesse formato, a vírgula separa a parte decimal, e o ponto só pode separar milhares em grupos de três: `numero("1.500")` devolve 1500, enquanto `numero("2.5")` produz um erro com a sugestão de escrever `2,5` ou usar `define_opcao(:formato_numero, :en)`. No formato en-US, o ponto separa a parte decimal, e a vírgula separa milhares.

Literais numéricos no código sempre usam ponto decimal. `escreva` apresenta um número isolado no formato ativo (`2,5`), mas usa a notação do código dentro de listas, mapas e pares: `escreva([1.5, 2])` produz `[1.5, 2]`. `como_texto(valor)` devolve o texto que `escreva` apresentaria.

**Aritmética:** por padrão, as operações usam aritmética decimal, selecionada por `define_opcao(:tipo_numero, :decimal)`. `define_opcao(:tipo_numero, :float)` seleciona operações em ponto flutuante binário segundo o padrão IEEE 754. Essa escolha é independente do formato de texto (`pt` ou `en`). Também pode ser feita pela variável de ambiente `EXPRESSA_ARITMETICA` ou pela opção de linha de comando `--aritmetica decimal|float`.

**Acesso:** `mat::soma` acessa um nome em um módulo; `pessoa:nome`, uma chave de mapa; `p:chave` e `p:valor`, os campos de um par. A chamada `xs.tamanho()` equivale a `tamanho(xs)`: o valor antes do ponto é passado como primeiro argumento. Essa forma de chamada também é chamada de UFCS nesta especificação.

**Quebra de linha:** dentro de parênteses e colchetes, uma expressão pode ocupar várias linhas. Fora deles, a quebra de linha encerra a expressão. Um operador no **fim** da linha, como `a +` seguido de `2` na linha seguinte, produz erro; para continuar a conta em outra linha, coloque a expressão entre parênteses.

No **começo** de uma linha, `+`, `*`, `/`, `%`, comparações, `e`, `ou`, `->`, `se_falhar`, `.` e `::` não podem continuar a expressão anterior. Já `-`, `(`, `[` e `:` iniciam outro comando. Por exemplo, `total = 10` seguido de `- 3` em outra linha não subtrai 3 de `total`: a segunda linha é uma expressão independente. Se seu valor for descartado antes da última linha, ocorre o erro de valor não usado.

Dentro de um bloco delimitado por `{` e `}` ou por `inicio` e `fim`, as quebras de linha separam comandos, mesmo quando o bloco está dentro de parênteses. As quebras entre palavras-chave continuam permitidas: entre `se cond` e o início do bloco, entre o fim de um ramo e `senao` ou `ou se`, entre `funcao(x)` e seu corpo, ou entre `x se_falhar` e o bloco de tratamento. Dois comandos na mesma linha produzem erro.

```text
10 + 5
x > 10 e x < 20
nao verdadeiro
```

---

## 5. Blocos como expressões

```text
resultado = inicio
    10 + 5
fim
// resultado = 15

resultado = { 10 + 5 }    // igual
```

---

## 5.1. Matrizes

Matrizes são retangulares, contêm apenas números e são criadas pelo módulo `matriz`. Suas células são armazenadas em ponto flutuante binário (`f64`).

No modo decimal, pequenas diferenças de representação binária em torno de um número com até 15 algarismos significativos são eliminadas na conversão para `numero`. Assim, a célula resultante de `3 * matriz([[0.1]])` é apresentada como `0,3`. A mesma regra vale para resultados de `mat`, `cartesiano` e `tela_grafica`: `seno(radianos(30))` produz `0,5`. Outros resultados preservam seus dígitos, como `raiz(2)`, que produz `1,4142135623730951`. No modo `float`, essa correção não é aplicada. Um resultado que não possa ser representado pelo tipo `numero` produz erro, inclusive em operações com matrizes.

Por padrão, o ambiente de execução limita cada matriz a 1.048.576 células. Esse limite pode ser configurado pela API Rust `ExecutionOptions::max_matrix_cells` e se aplica a cada matriz, não à memória total usada pelo programa. Os índices começam em 0: `A[linha, coluna]` acessa uma célula, e `A[i]` devolve uma linha como lista.

`xy(A, x, y)` (UFCS `A.xy(x, y)`) equivale a `A[y, x]`. `xy(A, x, y, valor)` altera essa mesma célula e devolve a própria matriz, permitindo encadear chamadas. São funções do módulo `matriz`: x é coluna, y é linha, ambos inteiros em base zero, com os mesmos limites e erros da indexação. O valor escrito deve ser numérico; outras referências à matriz observam a alteração.

```text
importe "matriz"

A = matriz([
    [1, 2, 3],
    [4, 5, 6],
])

escreva(A[0, 1]) // 2
A.xy(1, 0)       // também 2: equivale a A[0, 1]
A.xy(1, 0, 9)    // equivale a A[0, 1] = 9; devolve A
escreva(A + A)   // soma de matrizes com as mesmas dimensões
escreva(3 * A)   // multiplicação por um número
escreva(A * transposta(A))  // produto de matrizes com dimensões compatíveis
transposta(A)    // ou A.transposta()
det(matriz([[1, 2], [3, 4]]))  // determinante: matrizes 1×1, 2×2 ou 3×3
identidade(3)
zeros(2, 5)      // só zeros; uns(2, 5); cheia(2, 5, 7)
linhas(A)        // 2
colunas(A)       // 3
tamanho(A)       // também linhas
```

O construtor `matriz(linhas)` recebe uma lista de listas de números; todas as linhas devem ter o mesmo tamanho. Depois de `importe "matriz"`, o nome `matriz` identifica a função construtora. Nesse caso, use `A.transposta()`. Para escrever `A.matriz::transposta()`, importe o módulo com um apelido: `matriz = importe "matriz"`. O construtor passa a ser acessado por `matriz::matriz([[…]])`.

---

## 6. Condicionais

Cada ramo tem seu próprio bloco (`inicio`/`fim` ou `{`/`}`).

Quando `se` é usado como **comando**, `senao` é opcional. Quando seu resultado é usado diretamente em uma atribuição (`x = se …`), como argumento, como operando, como item de lista ou em `retorne se …`, o ramo `senao` é obrigatório.

Um `se` na última linha de um bloco continua sendo tratado como comando, inclusive no corpo de uma função, em `r = { … }` ou no tratamento de `se_falhar`. Nesse caso, pode omitir `senao`; se nenhum ramo for executado, o resultado será `nada`.

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
para i em 1..=5
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

A condição de `enquanto` deve resultar em `verdadeiro` ou `falso`. O corpo é delimitado por `inicio` e `fim` ou por `{` e `}`.

`para x em` percorre **listas**, **textos** (um caractere por vez), **conjuntos**, **mapas** (pares com os campos `p:chave` e `p:valor`, na ordem de exibição) e **intervalos**. `para i em 5` é inválido; use `0..5` ou `1..=5`. O intervalo de `para i em 0..` não tem limite final: o programa precisa interromper o laço, por exemplo com `pare`. Se houver um limite de tempo configurado na máquina virtual, ele também pode interromper a execução.

A forma antiga `para i de … até …` não é aceita.

```text
para ch em "olá"
inicio
    escreva(ch)        // o  l  á
fim
```

`pare` encerra o laço mais interno (`para`, `enquanto` ou `repita`). `continue` inicia a próxima iteração desse laço. Usar esses comandos fora de um laço produz erro. Eles não são capturados por `se_falhar`.

---

## 8. Funções como valores

Funções são valores de primeira classe: podem ser atribuídas a variáveis, passadas como argumentos e devolvidas por outras funções.

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

`retorne` só pode ser usado **dentro de uma função**. Sem argumento, devolve `nada`, assim como `retorne nada`. Quando houver um valor de retorno, ele deve começar **na mesma linha** de `retorne`. Esse comando não é capturado por `se_falhar`.

As mesmas regras de condicionais (§6) se aplicam dentro das funções: um `se` usado diretamente como valor exige `senao`; como comando, pode omiti-lo.

```text
se n < 0 { escreva("negativo") }           // ok
x = se n >= 7 { "ok" }                     // erro: falta senao
x = se n >= 7 { "ok" } senao { "não" }     // ok
```

---

## 9. Listas

- Indexação começa em **0**
- Criação: `[1, 2, 3]`

Pela regra de quebra de linha (§4), o `[` usado para acessar um elemento ou uma fatia deve ficar na mesma linha do valor acessado (`numeros[0]`, `obtenha_lista()[0]`), exceto dentro de parênteses. Um `[` no começo da linha inicia uma nova lista. O conteúdo entre colchetes pode ocupar várias linhas. A regra também vale para textos e matrizes.

As operações a seguir ilustram consultas independentes. Os comentários indicam o resultado de cada consulta; o trecho não é um programa completo.

```text
numeros = [10, 20, 30, 40]

tamanho(numeros)               // 4
numeros[0]                     // 10
numeros + [50]                 // [10, 20, 30, 40, 50]
numeros.remova(1)              // sem o 2º item (cópia)
numeros.remova(2..4)           // sem os índices 2 e 3 (fim exclusivo)
numeros.remova(2..=3)          // igual a remova(2..4)
numeros contem 20              // verdadeiro
primeiro(numeros)              // 10 (primeiro("olá") é "o")
ultimo(numeros)                // 40
numeros[1..3]                  // [20, 30]
numeros[1..]                   // do índice 1 até o fim: [20, 30, 40]
"b"[0..2]                      // "b" (corta no tamanho)
ordene([3, 1, 2])              // [1, 2, 3] (cópia)
ordene(pessoas, funcao(p) { p:idade })   // por chave
numeros += [50]                // igual a numeros = numeros + [50]; agora há cinco itens
```

`ordene` compara como `<`: itens (ou chaves) todos números ou todos textos; misturar é erro. Itens com a mesma chave mantêm a ordem original.

Índices de **texto** também começam em **0** e se referem a caracteres Unicode, não a bytes. `t[1]` lê um caractere. `t[1] = "b"` substitui esse caractere e reatribui o texto; o novo valor deve ser um texto de tamanho 1. O tamanho do texto permanece o mesmo. Depois de `a = t`, uma atribuição como `t[0] = "x"` preserva o texto anterior em `a`.

```text
s = "ABCDE"
s[1] = "b"                     // "AbCDE"
linhas[0][1] = "x"             // se linhas[0] é texto, reescreve o caractere
```

---

### 9.1. Cópias e referências

A atribuição `a = b` e a passagem de argumentos compartilham listas, mapas, conjuntos, matrizes e imagens. Uma alteração no objeto compartilhado é observada por todas as referências a ele. Textos têm comportamento de valor: substituir um caractere reatribui o texto e preserva as cópias anteriores.

Operações como `+`, `remova` e `ordene` criam cópias superficiais de coleções: a coleção externa é nova, mas objetos mutáveis contidos nela continuam compartilhados. A leitura `A[i]` devolve uma lista independente da matriz.

`clone(valor)` faz uma cópia profunda dos dados clonáveis, preservando ciclos e referências compartilhadas entre os elementos da cópia. Para que uma função possa ser clonada, a análise do interpretador deve comprovar que ela não depende de variáveis externas nem de outras formas de acesso ao ambiente original. Essa análise é conservadora: algumas funções são recusadas quando não é possível comprovar essa independência. Módulos, processos e recursos não são clonáveis. Consulte as [regras completas de clonagem](clone.md).

```text
xs = [[1]]
ys = xs + []
ys[0][0] = 2
escreva(xs[0][0])    // 2: o elemento aninhado é compartilhado
```

### 9.2. Intervalos

`0..3` representa um intervalo que gera inteiros, não uma lista. O limite final de `..` é **exclusivo**: `0..3` gera 0, 1 e 2. O operador `..=` inclui o limite final: `0..=2` representa o mesmo intervalo que `0..3`.

Quando o início é omitido, seu valor é 0: `..3` equivale a `0..3`. Quando o limite final é omitido, o intervalo não tem fim: `0..` e `..` começam em 0; `5..` começa em 5. Intervalos equivalentes são iguais (`1..3 == 1..=2`), e `escreva` usa a representação com limite exclusivo: `escreva(0..=2)` produz `0..3`.

Os limites são inteiros de 64 bits e são avaliados uma única vez. `n..=m` equivale a `n..(m+1)`; se `m+1` ultrapassar o maior inteiro de 64 bits, ocorre um erro. O incremento é sempre 1; não há parâmetro para alterá-lo. Intervalos vazios, como `3..3` e `3..1`, não geram valores. Fora de parênteses, o limite final, quando informado, deve ficar na mesma linha do operador `..` (§4).

As funções do núcleo também aceitam intervalos. `tamanho(3..5)` devolve 2; `primeiro(3..5)`, 3; e `ultimo(3..5)`, 4, o último valor gerado. Em um intervalo sem limite final, `primeiro(0..)` devolve 0, mas `tamanho(0..)` e `ultimo(0..)` produzem erros tratáveis com `se_falhar`. `eh_infinito(r)` (também `é_infinito`) devolve `verdadeiro` quando o intervalo não tem limite final; argumentos de outros tipos produzem erro. Um intervalo finito e não vazio pode ser reconstruído por `primeiro(r)..(ultimo(r)+1)`.

A igualdade compara o início e o limite final exclusivo, inclusive em intervalos vazios: `3..3 != 5..5`. Os únicos operadores de comparação disponíveis entre intervalos são `==` e `!=`. Não é permitido acessar um intervalo por índice nem somar intervalos com `+`. A expressão `r contem n` indica se o intervalo gera `n`: `(0..10) contem 3` é verdadeiro; `(0..3) contem 1.5` é falso, pois 1,5 não é inteiro. Um operando que não seja numérico produz erro.

Fatias e `remova` usam os mesmos valores do tipo `intervalo`. A fatia `xs[0..2]` seleciona os índices 0 e 1. Se o limite final ultrapassar o tamanho da coleção, a fatia termina no último elemento: `xs[0..1000]` devolve os cinco elementos de uma lista de tamanho 5.

Sem limite final, a fatia segue até o fim da lista ou do texto: `"thiago"[0..]` produz `"thiago"`, `"thiago"[2..]` produz `"iago"`, e `xs[..]` seleciona a coleção inteira. O intervalo também pode estar em uma variável: depois de `r = 2..`, use `xs[r]`. Um início negativo produz erro; um início além do tamanho da coleção produz uma fatia vazia.

Ao contrário das fatias, `remova` exige um intervalo com limite final: `xs.remova(1..)` produz erro. O intervalo `0..` continua sem tamanho definido; é a coleção que determina onde termina uma fatia que o utiliza.

```text
para i em 0..tamanho(xs) { escreva(xs[i]) }
para i em 1..=10 { escreva(i) }
xs[0..=2]                      // [10, 20, 30]
```

## 10. Pares e mapas

`chave -> valor` cria um **par**, com os campos `p:chave` e `p:valor`. A chave pode ser um texto, um número inteiro ou um booleano. A notação `:nome` equivale ao texto `"nome"`; palavras-chave e textos com espaços devem ser escritos entre aspas. Um par não pode ser usado como chave de mapa nem como elemento de conjunto.

Pares são copiados por valor. Quando um par contém outros pares, a leitura copia essa cadeia. Assim, repetir a atribuição `x = 1 -> x` tem custo total quadrático em relação ao número de repetições: cada novo par contém uma cópia da cadeia anterior.

`mapa` e `conjunto` são funções do núcleo. `mapa()` (ou `mapa([])`) é o mapa vazio. `mapa(xs)` recebe **uma** lista de pares; chaves duplicadas são erro. `m + (chave -> valor)` é um mapa novo com essa chave (`m += :c -> 3`).

Mapas usam tabelas hash, sem preservar a ordem de inserção. A igualdade compara as chaves e seus valores, independentemente da ordem. A impressão organiza as chaves por tipo (texto, inteiro, bool), em ordem crescente dentro de cada tipo. Essa ordenação da impressão não altera a ordem interna da tabela. `para p em m` percorre os pares (`p:chave`, `p:valor`) nessa mesma ordem; `chaves_de(m)` e `valores_de(m)` devolvem listas na mesma ordem.

```text
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

Fora de parênteses, as partes de `pessoa:nome` devem estar **na mesma linha** (§4). Depois de uma quebra de linha, `:nome` inicia outro comando e representa o texto `"nome"`, não o acesso a um campo:

```text
pessoa:nome          // campo
opções
:args                // não é opções:args
```

Para usar um campo como valor de um par, escreva `:op -> (obj:campo)`. As chaves `{` e `}` e os delimitadores `inicio` e `fim` representam blocos, não mapas. Colchetes representam listas, com vírgulas obrigatórias entre os elementos; a vírgula depois do último elemento é opcional.

---

## 11. Conjuntos

Um conjunto contém valores únicos, que podem ser textos, números inteiros ou booleanos. Seus elementos não têm índices. `conjunto()` e `conjunto([])` criam um conjunto vazio. `conjunto(xs)` recebe uma lista e descarta elementos repetidos.

O armazenamento usa uma tabela hash, e a ordem de iteração de `para` não é garantida. A exibição ordena os elementos pelos mesmos critérios usados para as chaves de mapas. A igualdade depende apenas dos elementos, não da ordem.

```text
s = conjunto([:ana, :bia, 1])
s contem :ana              // verdadeiro
s += :carlos               // acrescenta um elemento
s += conjunto([:bia, :dani])    // união
s = s.remova(:ana)         // por valor, devolve cópia
tamanho(s)
para x em s { escreva(x) }
```

Listas e mapas não são convertidos automaticamente em conjuntos. Em um conjunto, o argumento de `remova` é o elemento a remover, não um índice.

---

## 12. Textos

```text
nome = "  Maria Silva  "
nome[0]                    // um caractere
nome[1..3]                 // limite final exclusivo, limitado ao tamanho do texto
nome[1..]                  // do índice 1 até o fim

tamanho(nome)
maiuscula(nome)            // também maiúscula
sem_acento("olá")          // "ola"
minuscula(nome)
nome contem "Silva"
"ábaco" > "zebra"         // verdadeiro: ordem de ponto de código
procure(nome, "Silva")    // índice 0…n−1; nada se não achar
substitua(nome, "Maria", "Ana")
separe("a,b,c", ",")
junte(["a", "b"], " - ")
limpe(nome)
comeca_com(nome, "  Ma")   // também começa_com
termina_com(nome, "  ")
formate("{:<8} {:>5}", "Ana", 10)   // < esquerda  > direita  ^ centro
                               // mapa com :_como_texto (função de 1 arg → texto) usa essa função
nome[0]
nome[0..4]
```

---

## 13. Entrada, saída e funções do ambiente

`leia()` lê uma linha da entrada padrão, normalmente o teclado, e devolve o texto sem a quebra de linha.

`leia(mensagem)` apresenta a mensagem e, em seguida, espera a entrada de uma linha.

```text
nome = leia("Qual o seu nome? ")
escreva("Olá, " + nome)
```

Se a entrada terminar antes da leitura de uma linha, `leia()` produz um erro tratável com `se_falhar`. No terminal, o fim da entrada pode ser indicado por Ctrl+D ou Ctrl+Z, conforme o sistema.

`leia_linhas()` lê **todas** as linhas restantes da entrada padrão e devolve uma lista. Nesse caso, chegar ao fim da entrada não é um erro: um arquivo vazio resulta em `[]`.

```text
busca = argumentos[0]
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

`eh_terminal()` (também `é_terminal()`), no módulo `tela`, devolve `verdadeiro` quando a entrada vem de um terminal e `falso` quando vem de um arquivo ou de um encadeamento de comandos (pipe).

### 13.1. Argumentos e avaliação de código

`argumentos` é a lista dos textos fornecidos depois do nome do programa na linha de comando. `argumentos[0]` é o primeiro deles; sem argumentos adicionais, a lista é `[]`. O nome `argumentos` não pode ser reatribuído, mas os elementos da lista podem ser alterados.

`avalie(texto)` executa o texto como código Expressa no escopo atual e devolve o último valor. `avalie("2 + 3 * 4")` vale `14`. Erros de sintaxe ou de execução (por exemplo divisão por zero) são capturáveis com `se_falhar`.

### 13.2. Consulta de módulos e funções

`modulos()` (também `módulos()`) exibe o núcleo, os módulos nativos disponíveis e os arquivos já importados. Um mesmo arquivo aparece uma única vez, mesmo que tenha sido importado com vários apelidos. O primeiro caminho usado na importação identifica o arquivo; em caso de conflito de nomes, são usados caminhos absolutos.

`modulos(:dados)` devolve essas informações como uma lista de mapas. Cada mapa tem os campos `:nome` (texto) e `:funcoes` (lista estruturada de funções).

`funcoes()` (também `funções()`) exibe as funções públicas visíveis. `funcoes(alvo)` consulta um módulo, identificado por nome ou pelo próprio objeto. O valor `nada` indica o escopo atual, e `"nucleo"` indica o núcleo.

`funcoes(alvo, :dados)` devolve uma lista de mapas com os campos `:nome` (texto), `:args` (lista de textos) e `:doc` (documentação completa, ou `""` quando ausente). As listas seguem ordem alfabética e não incluem funções cujo nome começa com `_`. As formas que apenas exibem informações devolvem `nada`. A chamada `funcoes(:dados)` é inválida; use `funcoes(nada, :dados)`.

O REPL não possui comandos especiais `ls`, `ajuda` ou `catalogo`. A consulta de ajuda é feita pela função `ajuda(alvo)`.

### 13.3. Saída de texto e controle do terminal

`escreva` envia texto para a saída padrão (stdout). `escreva_erro` envia texto para a saída de erro (stderr).

Com `importe "tela"`, `limpe_tela()` limpa a tela. Para animações, `posicao_cursor(0, 0, falso)` move o cursor para o canto superior esquerdo sem apagar o conteúdo, evitando a cintilação causada por limpezas sucessivas.

`tamanho_tela()` devolve as dimensões do terminal em caracteres, em um mapa com os campos `:linhas` e `:colunas`. Quando não há terminal disponível, usa 24 linhas e 80 colunas. `nova_linha` é um valor de texto, não uma função: contém `"\n"` no Unix e `"\r\n"` no Windows.

`quadro(texto)` move o cursor para o canto superior esquerdo, escreve o texto e apaga o conteúdo anterior que sobrar à direita de cada linha ou abaixo da última linha escrita. As quebras de linha são convertidas para `nova_linha` na saída.

`bloco(linha, coluna)` devolve um mapa com os campos `:linha` e `:coluna`, contados a partir de zero. `escreva_em(bloco, texto)`, também disponível como `bloco.escreva_em(texto)`, escreve uma linha na coluna indicada, incrementa `:linha` e devolve o mesmo mapa. O texto não pode conter `"\n"`.

`posicao_cursor()` (também `posição_cursor()`) consulta a posição de saída e devolve um mapa com os campos `:linha` e `:coluna`, contados a partir de zero. `posicao_cursor(pos)` move o cursor para a posição indicada nesse mapa, sem apagar o conteúdo da tela nem alterar o mapa.

`posicao_cursor(linha, coluna, mostre)` move o cursor para coordenadas inteiras não negativas. O argumento booleano `mostre` determina se os dados pendentes da saída devem ser enviados imediatamente ao terminal. Seu valor padrão é `verdadeiro`: `posicao_cursor(linha, coluna)` envia esses dados. Com `falso`, o movimento pode ser enviado junto com a próxima escrita. A consulta sem argumentos e o movimento com mapa também enviam os dados pendentes imediatamente.

`apague_linha()` apaga toda a linha atual e posiciona o cursor na coluna zero, sem deslocar as outras linhas. A antiga função `casa()` não está disponível.

Na Aula, `leia()` começa na posição de saída e substitui o restante da linha, preservando as linhas abaixo. Quebras visuais causadas pelo ajuste do texto à largura da janela não contam como novas linhas. No terminal, a posição é relativa à área visível. A consulta pode falhar quando não há terminal ou quando ele não responde à solicitação; no Unix, o tempo limite é de 500 ms. Uma posição armazenada pode deixar de corresponder ao mesmo conteúdo depois de uma rolagem.

`pinte(texto, frente)` e `pinte(texto, frente, fundo)` devolvem texto com sequências ANSI para aplicar as cores e restaurar a formatação ao final. `fundo(texto, cor)` altera apenas a cor de fundo; `negrito(texto)` aplica negrito. As cores disponíveis são `normal`, `preto`, `vermelho`, `verde`, `amarelo`, `azul`, `magenta`, `ciano` e `branco`.

### 13.4. Espera e variáveis de ambiente

Com `importe "sistema"`, `durma(segundos)` suspende a execução pelo tempo informado. Por exemplo, `durma(0.5)` espera meio segundo.

Também em `sistema`, `ambiente(nome)` devolve a variável de ambiente como texto (`ambiente("HOME")`). Se ela não existe, é erro: `ambiente("EDITOR") se_falhar { "vim" }`.

### 13.5. Datas e horários

Com `importe "crono"`, `agora()` devolve a data e a hora em um mapa com os campos `:ano`, `:mes`, `:dia`, `:hora`, `:minuto`, `:segundo`, `:dia_da_semana` (de 1 para domingo a 7 para sábado) e `:epoch` (segundos desde o início de 1970, em UTC). Em sistemas Unix, a data e a hora usam o fuso local; nos demais sistemas, usam UTC.

`hoje()` devolve a data atual à meia-noite. `data(ano, mes, dia)` e `data(ano, mes, dia, hora, minuto, segundo)` constroem mapas com a mesma estrutura. Datas inválidas produzem erros tratáveis com `se_falhar`.

`adicione(d, n, unidade)` acrescenta uma quantidade positiva ou negativa à data. A unidade pode ser `:dias`, `:meses`, `:anos`, `:horas`, `:minutos` ou `:segundos`. Ao adicionar meses ou anos, se o dia não existir no mês de destino, usa-se o último dia desse mês: 31 de janeiro mais um mês resulta no último dia de fevereiro.

`dias_entre(a, b)` devolve a diferença em dias de calendário, com sinal, ignorando os horários. `eh_bissexto(ano)` (também `é_bissexto`) aceita um ano ou um mapa de data.

`formate_data(data, modelo)` formata uma data usando `%Y`, `%m`, `%d`, `%H`, `%M`, `%S` e `%%`. O ano usa quatro dígitos; os demais campos numéricos usam dois. Outra letra depois de `%` produz erro. A data pode ser um mapa como o de `agora()` — bastam os campos de `:ano` a `:segundo` — ou um número de segundos desde o início de 1970. A função `adicione` de `crono` recebe três argumentos; a de `cartesiano`, dois.

### 13.6. Encerramento e erros explícitos

`saia()` encerra o programa com código de saída 0. `saia(1)` encerra com o código 1, indicando falha ao sistema operacional. O encerramento não é capturado por `se_falhar`.

`falhe(descricao)` gera um erro de execução com a descrição informada. Ela pode ser um texto ou um valor com a função de conversão `:_como_texto`. A pilha de chamadas aponta para o local da chamada; a função nativa não acrescenta uma entrada própria à pilha.

`garanta(condicao)` exige um valor booleano. Quando a condição é falsa, produz a mensagem `condição incorreta`, seguida do código-fonte da condição com os espaços em branco normalizados. Erros produzidos por `falhe` e `garanta` podem ser tratados com `se_falhar`.

```text
se nome == ""
inicio
    escreva_erro("nome vazio")
    saia(1)
fim
```

```text
escreva("ok")
escreva_erro("falhou")
```

### 13.7. Gráficos cartesianos

Com `importe "cartesiano"`, `documento()` cria um documento gráfico, representado por um mapa com o campo `:graficos`. Cada documento corresponde a uma janela. `grafico()` (também `gráfico()`) cria um plano de coordenadas x e y, com campos como `:xmin` e `:curvas`. `eixos(g, xmin, xmax, ymin, ymax)` define os limites dos eixos; os valores de y crescem para cima.

`plote(g, função, cor)` calcula aproximadamente 400 pontos entre `xmin` e `xmax`. Um resultado não finito interrompe o traçado naquele ponto. As cores são as mesmas do módulo `tela`. `adicione(c, g)` acrescenta ao documento uma cópia do gráfico; alterações posteriores em `g` não alteram essa cópia.

`mostre(c)` abre ou atualiza a janela do documento e a coloca em foco. A execução continua sem esperar que a janela seja fechada. Um documento sem gráficos produz erro. Cada chamada de `documento()` cria um documento independente. O tipo de um gráfico, consultado por `tipo(g)`, é `"mapa"`.

Se não houver ambiente gráfico disponível, `mostre` produz um erro tratável com `se_falhar`. Na linha de comando, os gráficos são exibidos por `expressa-grafico`, também acessível por `expressa --grafico`. Na Aula, aparecem no painel Gráficos; a ação **Destacar** abre uma janela separada.

### 13.8. Desenho em pixels

Com `importe "tela_grafica"`, `nova(largura, altura)` ou `nova(largura, altura, tamanho_pixel)` cria uma tela de pixels. Ela é representada por um mapa com os campos `:largura`, `:altura`, `:tamanho_pixel`, `:cor`, `:fundo` e `:ops`. A origem `(0, 0)` fica no canto superior esquerdo; x cresce para a direita, e y cresce para baixo.

A largura e a altura devem ser inteiros entre 1 e 2048. O argumento `tamanho_pixel`, de 1 a 16, define o lado do quadrado que representa cada pixel lógico na janela. Seu valor padrão é 1; com 2, cada pixel lógico ocupa um quadrado de 2×2 pixels. `evento`, `pixel` e `salve` usam as coordenadas da tela lógica.

As funções `cor`, `fundo`, `linha`, `circulo`, `ponto`, `retangulo`, `texto` e `apague` definem ou alteram o desenho. As cores são as mesmas do módulo `tela`, acrescidas de `transparente`, que permite áreas transparentes na imagem. `fundo(g, cor)` define a cor de fundo, inicialmente branca. `retangulo(g, x1, y1, x2, y2)` preenche um retângulo incluindo os dois cantos; as partes fora da tela são recortadas.

`como_imagem(g)` devolve uma cópia do desenho como valor do tipo `imagem`, com canal alfa zero nos pixels transparentes. `texto(g, x, y, t)` e `texto(g, x, y, t, tamanho)` desenham o texto na cor atual. A posição `(x, y)` indica seu canto superior esquerdo; `tamanho` é a altura da célula de cada caractere, com padrão 8. A sequência `\n` no texto inicia uma nova linha.

`imagem(g, caminho|img|tela, x, y, angulo)` desenha uma imagem na tela. A origem pode ser um caminho de PNG, um valor `imagem` ou outra tela convertida por `como_imagem`. O PNG é lido uma única vez. `pixel(g, x, y)` devolve o nome da cor naquela posição; coordenadas fora da tela produzem erro. `salve(g, "a.png")` grava um PNG com transparência sem abrir uma janela.

`mostre(g)` abre ou atualiza a janela da tela, separada do painel da Aula. `evento(g)` consulta a entrada sem esperar por novos eventos e consome os eventos pendentes dessa janela. O mapa devolvido contém `:teclas` (conjunto das teclas pressionadas), `:apertadas` e `:soltas` (alterações desde a consulta anterior), `:eventos` (lista na ordem em que ocorreram) e `:sair`. Cada item de `:eventos` tem os campos `:tipo`, `:tecla`, `:x`, `:y`, `:botao`, `:modificadores` e `:repeticao`.

Chame `mostre(g)` antes de consultar eventos e `evento(g)` uma vez por quadro. O ritmo pode ser controlado por `durma()` ou pelo laço gráfico. `laco_grafico(g)` (também `laço_gráfico`) cria esse laço. A chamada `l.tela_grafica::execute(config)` recebe um mapa com os campos obrigatórios `:fps`, `:eventos` (função com argumentos `ev` e `dt`) e `:desenho` (função com argumento `g`). `l.tela_grafica::pare()` (também `pare_laco`) encerra o laço depois da função de tratamento atual.

Cada chamada de `nova()` cria uma tela com sua própria janela. `tipo(g)` devolve `"mapa"`. Os detalhes de comunicação com a janela estão em [Tela gráfica](tela_grafica.md).

`feche(g)` solicita o fechamento da janela de `g` e a interrupção dos laços associados a essa tela. Devolve `g`, preserva o desenho e não afeta outras janelas. O fechamento é assíncrono; chamadas repetidas ou anteriores à primeira exibição são permitidas. Uma chamada posterior de `mostre(g)` pode abrir a janela novamente. `pare()` apenas interrompe o laço, sem fechar a janela.

### 13.9. Imagens

Com `importe "imagem"`, `carregue("a.png")` lê um PNG e devolve um valor do tipo `imagem`, com os pixels em memória. As dimensões máximas são 2048×2048. `salve(img, "b.png")` grava um PNG com transparência.

`rotacione(img, graus)` gira a imagem no sentido horário e devolve uma nova imagem; rotações de 90, 180 e 270 graus são exatas. `redimensione(img, fator)` e `redimensione(img, fx, fy)` alteram as dimensões usando o método do vizinho mais próximo. `largura(img)` e `altura(img)` devolvem as dimensões em pixels.

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

**Pastas e arquivos:**

```text
existe("dados.txt")              // arquivo ou pasta
eh_pasta("fotos")                // também é_pasta
liste("fotos")                   // nomes, em ordem, com os ocultos
liste("fotos", "*.png")          // * qualquer trecho, ? um caractere
crie_pasta("saida/2024")         // cria as que faltam; já existir não é erro
apague_arquivo("temp.txt")       // pasta é erro
apague_pasta("build")            // a pasta e tudo dentro; recusa / e a pasta pessoal
mova("a.txt", "arquivo/")        // move ou renomeia; para dentro se o destino é pasta
copie("a.txt", "a.bak")          // só arquivos; para dentro se o destino é pasta
```

**Manipulação de caminhos:** as quatro primeiras funções abaixo operam sobre o texto do caminho, sem consultar o sistema de arquivos. `pasta_atual()` consulta a pasta de execução do programa.

```text
junte_caminho("dados", "2024", "a.csv")   // "dados/2024/a.csv"; um trecho absoluto descarta os anteriores
nome_do_arquivo("fotos/a.png")            // "a.png"
pasta_de("fotos/a.png")                   // "fotos"; sem pasta, ""
extensao("a.tar.gz")                      // "gz" (sem ponto; sem extensão, ""); também extensão
pasta_atual()                             // pasta de onde o programa foi chamado
```

Caminhos relativos são resolvidos a partir da **pasta do arquivo `.lep`**. A função `pasta_atual()` informa a pasta de onde o programa foi chamado, que pode ser diferente. `~` e `~/pasta` usam a pasta pessoal indicada por `HOME` ou `USERPROFILE`; a forma `~nome` não é expandida.

Para interpretar um argumento de linha de comando como caminho relativo à pasta de chamada, use `junte_caminho(pasta_atual(), argumentos[0])`. Falhas como arquivo inexistente ou falta de permissão produzem erros tratáveis com `se_falhar`. Na Aula, os caminhos relativos ficam na pasta do aluno; a expansão de `~` continua indicando a pasta pessoal.

---

## 15. Tratamento de erros

- Um erro não tratado interrompe a execução e produz um diagnóstico com a localização do erro e a pilha de chamadas. Erros em programas Expressa devem ser diagnosticados, sem provocar uma falha interna do interpretador.
- Para tratar, usa-se `expressao se_falhar { alternativa }` ou `expressao se_falhar erro { tratamento }`. O bloco é obrigatório; as formas antigas `se_falhar valor` e `se_falhar(valor)` não são aceitas.
- O bloco só executa em caso de erro de execução; seu último valor é o resultado alternativo. Também aceita `inicio ... fim`, como os demais blocos.
- A variável que recebe a mensagem do erro é opcional, e seu nome é escolhido pelo programa. Seu valor é um texto, e ela só existe dentro do bloco de tratamento. Se tiver o mesmo nome de uma variável externa, oculta essa variável durante o tratamento, sem alterá-la.
- Erros no próprio tratamento se propagam e podem ser capturados por um `se_falhar` externo. `retorne`, `pare`, `continue` e `saia` não são erros capturáveis.
- `se_falhar` não desfaz atribuições nem outros efeitos ocorridos antes do erro. Uma importação sem apelido verifica todos os conflitos de nomes antes de disponibilizar qualquer nome no escopo. Essa verificação não desfaz os efeitos produzidos pela inicialização do módulo.

```text
importe "arquivo"
linhas = leia_arquivo("arquivo.txt") se_falhar { [] }

valor = 10 / 0 se_falhar { 0 }

item = lista[99] se_falhar { "não existe" }

valor = numero("inválido") se_falhar erro {
    escreva("Não foi possível converter:", erro)
    falso
}

garanta(n > 0)
se n < 0 { falhe("negativo") }
```

---

## 16. Módulos e importação

A mesma forma vale para arquivos `.lep` e para os módulos nativos `sistema`, `matriz`, `arquivo`, `tela`, `mat`, `crono`, `cartesiano`, `tela_grafica`, `imagem` e `processo`.

```text
mat = importe "matematica"     // apelido: acesso aos nomes por mat::
mat::soma(10, 5)

importe "matematica"           // disponibiliza os nomes no escopo atual
soma(10, 5)
```

Na importação sem apelido, os nomes do módulo são disponibilizados diretamente no escopo atual. Se algum deles já existir, seja no programa, no núcleo ou em outro módulo importado, ocorre um erro como ``já existe `fundo` neste escopo``. A importação não substitui o nome anterior. Importar o mesmo módulo duas vezes sem apelido também produz erro.

Para evitar conflitos, use um apelido: `g = importe "tela_grafica"`. Nesse caso, os nomes são acessados pelo módulo, como `g::fundo`, e não são introduzidos diretamente no escopo. Módulos com apelidos diferentes podem ter funções de mesmo nome, como `t::fundo` e `g::fundo`.

Nomes introduzidos por uma importação sem apelido não podem ser reatribuídos: `importe "mat"` seguido de `raiz = 1` produz erro, com a sugestão de usar um apelido. Parâmetros, variáveis de laço e variáveis de tratamento de `se_falhar` podem usar esses mesmos nomes em seus próprios escopos, ocultando temporariamente os nomes importados, assim como os do núcleo.

Os nomes `"tela_gráfica"` e `"tela_grafica"` identificam o mesmo módulo nativo.

Um nome **sem** `/`, `./` ou a extensão `.lep` é procurado primeiro entre os módulos nativos. Um caminho com `./`, `/` ou o sufixo `.lep` identifica um arquivo. Se um nome identificar tanto um módulo nativo quanto um arquivo no mesmo diretório, a importação é ambígua e produz erro. Por exemplo, se houver um arquivo `matriz.lep` ao lado do programa, `importe "matriz"` é inválido; use `importe "./matriz"` para importar o arquivo. Enquanto esse conflito existir, o módulo nativo não pode ser importado pelo nome `"matriz"`.

Depois de `importe "matriz"`, o nome `matriz` identifica a função construtora; use `A.transposta()`. Com o apelido `matriz = importe "matriz"`, use `matriz::matriz([[…]])` e `A.matriz::transposta()`.

---

## 17. Execução do programa

Os comandos do programa são executados **na ordem em que aparecem**. Não é necessário definir uma função `main`.

A chamada `expressa diretorio [argumentos…]` executa o arquivo `principal.lep` diretamente nesse diretório. Os argumentos são preservados e as importações são resolvidas a partir da pasta desse arquivo. Se `principal.lep` não existir, ocorre um erro; o interpretador não procura funções especiais nem outros candidatos.

`define_opcao(opcao, valor)` (também `define_opção`) configura toda a execução. `:tipo_numero` aceita `:decimal` ou `:float`; `:formato_numero`, `:pt` ou `:en`; e `:fechar_janelas`, `verdadeiro` ou `falso`. Os padrões são `:decimal`, `:pt` e `falso`, respectivamente; a linha de comando e o ambiente podem escolher os valores numéricos iniciais. As opções numéricas são independentes. Opções ou valores desconhecidos produzem erro, sem alterar a configuração anterior. A função devolve `nada` e substitui a antiga `formato`.

Quando `:fechar_janelas` está ativa no fim da execução, suas janelas gráficas e seus gráficos cartesianos são fechados automaticamente, inclusive em caso de erro ou interrupção. A política também abrange janelas abertas antes de ser ativada. No REPL, ela é aplicada ao encerrar a sessão.

Em sistemas Unix, arquivos `.lep` podem ser executados diretamente quando têm permissão de execução e começam com a linha de interpretador:

```text
#!/usr/bin/env expressa
escreva("olá")
```

```text
chmod +x ola.lep
./ola.lep
```

Os textos fornecidos depois do nome do arquivo ficam disponíveis em `argumentos`: na chamada `./ola.lep Thiago`, `argumentos[0]` contém `"Thiago"`.

---

## 18. Programas externos

O módulo `processo` executa programas externos e permite encadear sua saída com outros programas ou com funções da Expressa. Essas operações estão disponíveis na linha de comando. Na Aula, onde a execução é restrita à pasta do aluno, as chamadas produzem erro.

```text
importe "processo"

filtro = funcao(linha, oque) {
    se linha contem oque { linha }
}

r = execute("ls", [filtro, ".txt"], ["wc", "-l"])
escreva(r:saida)   // quantidade de nomes que contêm ".txt", como lista de linhas

para linha em linhas_de("tail -f app.log") {
    se linha contem "xxx" { escreva(linha) }
    se linha contem "FIM" { pare }       // sai do laço e encerra o tail
}
```

**Etapas.** `execute(comando, etapa, …)` e `linhas_de(comando, etapa, …)` recebem a mesma sequência. O **primeiro** argumento é um comando; os demais são comandos ou funções, e a saída de cada etapa entra na seguinte, linha a linha.

| Etapa | Significado |
|---|---|
| `"wc -l"` | Comando em texto: espaços separam argumentos; aspas duplas ou simples preservam espaços dentro de um argumento; `\|` encadeia comandos (`"sort \| uniq"`) |
| `["wc", "-l"]` | Comando em lista: cada item corresponde a um argumento; números são convertidos em texto |
| `filtro` / `maiuscula` | Função chamada com cada linha: `filtro(linha)` |
| `[filtro, ".txt"]` | Função com argumentos extras: `filtro(linha, ".txt")` |

O resultado de uma função determina a saída da etapa: `nada` descarta a linha; um texto substitui a linha; um número ou booleano é convertido em texto; e uma lista produz várias linhas. Outros tipos produzem erro. A notação de lista com uma função no primeiro item só representa uma etapa nesse contexto.

O texto do comando **não** é interpretado por um shell. Os caracteres `*`, `?`, `>`, `<`, `&`, `;`, `$` e `` ` `` fora de aspas produzem erro. Ao montar um comando com variáveis, prefira uma lista: `["grep", busca, arquivo]`. Para usar os recursos de um shell explicitamente, execute-o como um programa: `["sh", "-c", "ls *.txt"]`.

Dois comandos consecutivos, sem uma função intermediária, são conectados diretamente por um pipe do sistema operacional. As linhas recebidas pela Expressa não incluem os terminadores `\n` e `\r`. Sequências de bytes inválidas em UTF-8 são substituídas pelo caractere `�`.

**Opções.** As opções são informadas em um mapa no **último** argumento. Um mapa em outra posição ou uma chave desconhecida produz erro.

| Chave | Valor | Sem ela |
|---|---|---|
| `:pasta` | Pasta de execução dos comandos, relativa ao `.lep` | Pasta do `.lep` |
| `:entrada` | Texto ou lista de linhas para a entrada do primeiro comando | Entrada vazia |
| `:ambiente` | `mapa` de variáveis de ambiente extras | Herda o ambiente |
| `:tempo` | Limite em segundos (> 0); ao esgotá-lo, os comandos são encerrados e ocorre um erro | Sem limite próprio |
| `:exija` | `verdadeiro`: um código de saída diferente de 0 produz erro | `falso` |
| `:terminal` | `verdadeiro`: o comando usa o terminal (teclado, tela, cores) | `falso` |

Com `:exija`, a mensagem inclui o comando, o código de saída e as últimas linhas da saída de erro: `'git push' falhou (código 1): fatal: …`. Em `linhas_de`, a falha é informada no fim da iteração; interromper o laço antes, por exemplo com `pare`, não é considerado uma falha.

Com `:terminal`, o primeiro comando recebe entrada do teclado. A saída padrão do último comando e a saída de erro de todos os comandos vão diretamente para o terminal, sem serem coletadas pela Expressa. Esse modo permite executar programas interativos como `vim`, `ssh`, `sudo` e `less`, além de acompanhar cores e progresso de comandos como `cargo build`. Os campos `:saida` e `:erro` ficam vazios; `:codigo` continua informando o código de saída. A opção só é aceita por `execute`, sem etapas de função e sem `:entrada`.

```text
execute(["vim", "notas.txt"], mapa([:terminal -> verdadeiro]))
execute("cargo build --release", mapa([:terminal -> verdadeiro, :exija -> verdadeiro]))
```

**`execute`** espera todos os comandos terminarem e devolve um mapa:

| Campo | Valor |
|---|---|
| `:codigo` | Primeiro código diferente de 0, na ordem dos comandos; 0 se todos terminarem com sucesso |
| `:ok` | `:codigo == 0` |
| `:saida` | Lista das linhas que saíram da última etapa |
| `:erro` | Lista das linhas da saída de erro de todos os comandos |

Código diferente de 0 **não** é erro de execução; confira `:ok`. Um comando encerrado porque a etapa seguinte parou de ler (como `head -1`) não conta como falha.

**`linhas_de`** inicia a execução dos comandos e devolve um valor do tipo `"processo"`. A construção `para linha em …` recebe cada linha assim que ela passa por todas as etapas. Sair do laço por qualquer motivo (`pare`, `retorne`, erro ou `saia()`) encerra os comandos. Se todos terminarem, o laço também termina.

A saída de erro é encaminhada diretamente para `escreva_erro`. Sem `:exija`, um código de saída diferente de 0 não produz erro de execução; para consultar o código, use `execute`. Percorrer novamente um processo já encerrado não produz novas iterações. Valores do tipo `processo` não aceitam indexação nem funções de lista como `tamanho`.

**Erros.** Podem ser tratados com `se_falhar`: comando inexistente, falta de permissão, pasta de execução inexistente, tempo esgotado, etapa inválida e erro dentro de uma função de etapa. Neste último caso, os comandos também são encerrados. O comando `para` não é uma expressão e não pode ser seguido de `se_falhar`. A chamada que fornece os valores para o laço pode ter um tratamento: `para l em linhas_de("x") se_falhar { [] } { … }`. Esse tratamento se aplica à chamada, não a erros posteriores durante a iteração.

Sem `:terminal`, os comandos recebem apenas o conteúdo de `:entrada`, ou uma entrada vazia. Programas que precisam ler o teclado, como editores e solicitações de senha, exigem o modo de terminal. Alguns programas, como `grep`, acumulam a saída em blocos quando ela é enviada a um pipe; para acompanhar `tail -f` linha a linha, use `grep --line-buffered` ou uma função de filtro da Expressa.

---

## Exemplo completo

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
