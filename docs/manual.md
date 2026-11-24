# Manual da Expressa

Este manual reúne a sintaxe, as operações e as funções da Expressa para consulta durante a programação. Para aprender a linguagem passo a passo, comece pelos tutoriais.

| Texto | Papel |
|-------|--------|
| **Manual** | referência da linguagem e das funções |
| [Tutorial básico](tutoriais/basico.md) | primeiros passos, capítulos 1 a 8 e projetos |
| [Tutorial avançado](tutoriais/avancado.md) | coleções, funções, módulos, arquivos e gráficos |
| [Folha de consulta](folha-de-consulta.md) | resumo da sintaxe e das funções |
| [Especificação](especificacao-linguagem.md) | regras detalhadas para quem implementa a linguagem |

No expressa-aula, pressione **F1** para consultar a ajuda das funções. Você também pode chamar `ajuda("escreva")` em um programa ou no REPL, o modo interativo. As fichas usadas pela ajuda estão em `docs/nativas/` no repositório.

Os programas são salvos em arquivos com a extensão `.lep`. Os índices começam em **0**. Em uma coleção não vazia, o último índice é o tamanho menos um; índices negativos geram erro. Nas fatias, o índice final não é incluído: `[0..2]` contém os índices 0 e 1. Quando o índice final é omitido, como em `[2..]`, a fatia vai até o fim.

---

## Executar

```text
expressa                 REPL
expressa prog.lep        executa o arquivo
expressa prog.lep Ana    argumentos[0] é "Ana"
expressa debug prog.lep  depurador
```

No expressa-aula, clique em **Executar** ou pressione F5. Quando `leia` pedir uma resposta, digite no painel de saída e pressione Enter. Os valores do campo **Argumentos** ficam disponíveis na lista `argumentos`, a partir de `argumentos[0]`.

Em sistemas Unix, você pode colocar `#!/usr/bin/env expressa` na primeira linha do arquivo e executar `chmod +x prog.lep` no terminal. Depois, use `./prog.lep` para executar o programa.

---

## Tipos

| Tipo | Exemplo | Notas |
|------|---------|--------|
| número | `10` `3.14` `-8` | no código, ponto; valor decimal exato (`0.1+0.2` é `0.3`) |
| texto | `"olá"` | |
| bool | `verdadeiro` `falso` | valor lógico |
| intervalo | `0..3` `1..=3` | sequência de inteiros |
| lista | `[1, 2, 3]` | `xs[0]` é o primeiro |
| par | `:nome -> "Ana"` | `p:chave` `p:valor` |
| mapa | `mapa([:nome -> "Ana"])` | `pessoa:nome` |
| conjunto | `conjunto([:ana, 1])` | valores únicos |
| matriz | `matriz([[1, 2], [3, 4]])` | só números; `importe "matriz"` |
| imagem | valor retornado por `carregue("figura.png")` | imagem em memória; `importe "imagem"` |
| função | `função(x) { x + 1 }` | também `funcao` |
| módulo | `importe "mat"` | |
| processo | `linhas_de("ls")` | programa em execução; `importe "processo"` |
| nada | `nada` | ausência de valor; `escreva` também devolve nada |

`nada` é um literal reservado, como `verdadeiro` e `falso`: `resultado = nada`, `resultado == nada` e `retorne nada`. Seu tipo é `"nada"`. Não pode ser usado como nome de variável ou parâmetro.

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

Cada comando fica em uma linha; dois comandos na mesma linha são erro. Dentro de `( )` e `[ ]` a quebra de linha é livre. Fora deles, a quebra de linha termina a expressão: para quebrar uma conta longa, coloque-a entre parênteses. Um operador no fim ou no começo da linha (`+`, `e`, `->`, `.` …) é erro. As palavras-chave podem ir para a linha seguinte: `se cond` e depois `inicio`, `fim` e depois `senao` ou `ou se`.

```text
total = (preco * quantidade
         + frete)
```

Um valor calculado e não usado, sem efeito, numa linha que não é a última do bloco é erro: `x + 1` sozinho na linha não faz nada. Chamadas de função, blocos e `se` podem ficar sozinhos na linha.

```text
// uma linha
/* várias linhas */
```

`#` não inicia um comentário comum; ele só é aceito na primeira linha, na forma `#!`.

Uma função pode ler variáveis de um escopo externo, mas não pode reatribuir essas variáveis. Ela pode alterar elementos de listas, mapas e matrizes compartilhados com o escopo externo.

---

## Operadores

### Cálculos

| Operador | Operação | Exemplo |
|----------|----------|---------|
| `+` | soma | `2 + 3` → `5` |
| `-` | subtração | `8 - 3` → `5` |
| `*` | multiplicação | `4 * 3` → `12` |
| `/` | divisão | `7 / 2` → `3,5` no formato padrão |
| `%` | resto da divisão | `7 % 3` → `1`; `-7 % 3` → `-1` |
| `-valor` | troca o sinal | `-(-3)` → `3` |

`+` também junta textos (`"Idade: " + 10`) e listas (`[1, 2] + [3]`). Em conjuntos, faz a união de dois conjuntos ou acrescenta um elemento: `s + :ana`. Em mapas, `m + (chave -> valor)` é um mapa novo com essa chave. As operações `+`, `-` e `*` com matrizes estão descritas na seção [matriz](#importe-matriz).

### Comparações

O resultado de uma comparação é `verdadeiro` ou `falso`.

| Operador | Significado | Exemplo |
|----------|-------------|---------|
| `==` | igual | `resultado == nada` |
| `!=` | diferente | `nome != ""` |
| `<` / `<=` | menor / menor ou igual | `idade < 18` |
| `>` / `>=` | maior / maior ou igual | `nota >= 7` |

As comparações de ordem (`<`, `<=`, `>` e `>=`) aceitam dois números ou dois textos. Textos seguem a ordem dos pontos de código Unicode: `"ábaco" > "zebra"` é `verdadeiro`.

A forma `a < b < c` é inválida. Use `a < b e b < c`.

### Condições lógicas

| Operador | Significado | Exemplo |
|----------|-------------|---------|
| `e` | as duas condições são verdadeiras | `nota >= 7 e faltas <= 5` |
| `ou` | pelo menos uma condição é verdadeira | `idade < 18 ou estudante` |
| `nao` / `não` | inverte o valor lógico | `não encontrado` |

Os operandos devem ser valores lógicos. Em `a e b`, `b` só é avaliado se `a` for verdadeiro. Em `a ou b`, `b` só é avaliado se `a` for falso.

### Pertencimento: `contem` / `contém`

| Valor à esquerda | O que procura | Exemplo |
|------------------|---------------|---------|
| lista | um item | `[10, 20] contem 20` |
| texto | um trecho de texto | `"Maria Silva" contem "Silva"` |
| mapa | uma chave | `pessoa contem :idade` |
| conjunto | um elemento | `nomes contem :ana` |
| intervalo | um número que ele gera | `(0..10) contem 3` |

O resultado é `verdadeiro` ou `falso`. Em um intervalo, um número que não é inteiro dá `falso`, e um valor que não é número gera erro.

### Atribuição e atualização

| Forma | Significado | Exemplo |
|-------|-------------|---------|
| `alvo = valor` | guarda ou substitui o valor | `idade = 18` |
| `alvo += valor` | soma ou junta ao valor atual | `nomes += ["Ana"]` |
| `alvo -= valor` | subtrai do valor atual | `saldo -= 10` |

`+=` aceita os mesmos tipos de `+`; `-=` aceita os mesmos tipos de `-`. Por exemplo, `nome += " Silva"` acrescenta texto, `s += :ana` acrescenta um elemento ao conjunto e `m += :cidade -> "Natal"` acrescenta uma chave ao mapa. O alvo também pode ser um elemento, como em `xs[0] += 1`; o receptor e seus índices são avaliados uma única vez.

### Intervalos, pares e acesso

| Forma | Significado | Exemplo |
|-------|-------------|---------|
| `a..b` | intervalo com fim exclusivo | `0..3` gera `0`, `1`, `2` |
| `a..=b` | intervalo que inclui o fim | `1..=3` gera `1`, `2`, `3` |
| `chave -> valor` | cria um par | `:nome -> "Ana"` |
| `colecao[indice]` | acessa um elemento | `xs[0]` |
| `colecao[a..b]` | obtém uma fatia | `"Maria"[0..3]` → `"Mar"` |
| `mapa:chave` | acessa o valor de uma chave | `pessoa:nome` |
| `modulo::nome` | acessa um nome do módulo | `mat::raiz(9)` após `mat = importe "mat"` |
| `valor.f(...)` | passa o valor como primeiro argumento | `xs.tamanho()` equivale a `tamanho(xs)` |

Índices começam em zero. Intervalos também podem omitir o começo (`..3` começa em zero) ou o fim (`2..` não tem teto). Os parênteses são obrigatórios nas chamadas: `xs.tamanho()`.

### Prioridade e parênteses

Operações nas primeiras linhas da tabela têm prioridade sobre as seguintes. Use parênteses para agrupar uma expressão.

| Prioridade | Operações |
|------------|-----------|
| 1 — maior | chamadas e acessos: `f(x)`, `xs[i]`, `pessoa:nome`, `mat::raiz`, `xs.tamanho()` |
| 2 | sinal negativo e negação: `-valor`, `nao`, `não` |
| 3 | `*`, `/`, `%` |
| 4 | `+`, `-` |
| 5 | `..`, `..=` |
| 6 | `==`, `!=`, `<`, `<=`, `>`, `>=`, `contem`, `contém` |
| 7 | `e` |
| 8 | `ou` |
| 9 | `->` |
| 10 — menor | `se_falhar` (veja [Erros](#erros)) |

```text
2 + 3 * 4                // 14
(2 + 3) * 4              // 20
nao (idade == 18)         // inverte o resultado da comparação
```

`nao` tem prioridade sobre `==`: para negar uma comparação, agrupe-a como no último exemplo.

---

## Controle de fluxo

```expressa
situacao = se nota >= 7 { "aprovado" } senão { "reprovado" }
se n < 0 { escreva("negativo") }     // senão é opcional como comando

se nota == 10 {
    escreva("gabaritou")
} ou se nota >= 7 {
    escreva("aprovado")
} senão {
    escreva("reprovado")
}

repita 3 vezes { escreva("olá") }
para i em 1..=5 { escreva(i) }
para nome em ["Ana", "Bruno"] { escreva(nome) }
para ch em "olá" { escreva(ch) }
para p em mapa([:a -> 1]) { escreva(p:chave, p:valor) }

i = 1
enquanto i <= 3 {
    escreva(i)
    i += 1
}
pare          // sai do laço
continue      // próxima volta
```

Quando o resultado de `se` é usado diretamente como **valor** (`x = se …`, argumento, `retorne se …`), o `senão` é obrigatório. Como **comando**, `senão` é opcional. Um `se` na última linha de um bloco, como o corpo de uma função, é comando: sem `senão` e sem ramo executado, o bloco vale `nada`. `para … em` percorre listas, textos, conjuntos, intervalos e mapas; em um mapa, cada volta recebe um par (`p:chave`, `p:valor`) na ordem da impressão. Em uma sequência com `ou se`, apenas o primeiro bloco cuja condição for verdadeira é executado. `pare` e `continue` só podem ser usados dentro de laços.

`retorne x` encerra a função imediatamente e retorna o valor de `x`. Só pode ser usado dentro de uma função. Sem valor, `retorne` retorna `nada`. Também é possível escrever `retorne nada`.

---

## Funções

O valor da última expressão do bloco é o resultado da função.

```expressa
soma = função(x, y) { x + y }
escreva(soma(10, 5))      // 15

busca = função(xs, alvo) {
    para x em xs {
        se x == alvo { retorne verdadeiro }
    }
    falso
}
```

`função` e `funcao` são formas equivalentes. Você pode chamar uma função de duas formas:
1. `tamanho(xs)`
2. `xs.tamanho()`

Na segunda forma, o primeiro argumento vem antes do nome da função, seguido de um ponto; os demais argumentos ficam entre os parênteses.

```expressa
expressa> comeca_com("Hoje eu acordei", "Hoje")
verdadeiro
expressa> "Hoje eu acordei".comeca_com("Hoje")
verdadeiro
```


---

## Módulos

```expressa
mat = importe "matematica"
mat::soma(10, 5)

importe "matematica"      // funções disponíveis sem prefixo
soma(10, 5)
```

Os módulos nativos incluem `tela`, `mat`, `matriz`, `arquivo`, `sistema`, `crono`, `cartesiano`, `tela_grafica`, `imagem` e `processo`. Eles fazem parte da linguagem.

Em uma importação sem `/` nem `.lep`, a Expressa procura um módulo nativo. Se houver um arquivo local com o mesmo nome, a importação é ambígua e gera erro. Use `importe "./matriz"` para escolher explicitamente o arquivo local. Os caminhos de importação são relativos à pasta do arquivo que faz o `importe`.

Sem apelido, os nomes do módulo ficam disponíveis diretamente no escopo da importação. Se algum desses nomes já estiver em uso (no programa, no núcleo ou em outro módulo), ocorre um erro. Depois da importação, esses nomes também não podem ser reatribuídos: `importe "mat"` seguido de `raiz = 1` é erro. Use um apelido para evitar o conflito, como em `g = importe "tela_grafica"`; com apelido, nada é espalhado no escopo. Parâmetros e variáveis de laço podem usar esses nomes. `importe "tela_gráfica"` também funciona.

---

## Erros

Um erro não tratado encerra o programa. Com `se_falhar`, o bloco de tratamento é executado quando a expressão falha, e seu resultado substitui o da expressão.

```expressa
valor = 10 / 0 se_falhar { 0 }
importe "arquivo"
linhas = leia_arquivo("x.txt") se_falhar { [] }
```

Para acessar a mensagem do erro, coloque um nome entre `se_falhar` e o bloco. Essa variável recebe a mensagem como texto e só existe dentro do bloco:

```expressa
valor = numero("abc") se_falhar erro {
    escreva("Erro ao converter um número:", erro)
    falso
}
```

O bloco é obrigatório, inclusive sem captura: `numero("abc") se_falhar { falso }`.
Se o próprio bloco de tratamento gerar outro erro, esse novo erro precisa ser tratado por outro `se_falhar`; caso contrário, ele encerra o programa.

`saia(1)` não é um erro, portanto **não** é capturado por `se_falhar`.

`falhe("negativo")` gera um erro com a mensagem informada e indica o local da chamada. 

`garanta(n > 0)` gera um erro se a condição for falsa, com a mensagem `condição incorreta n > 0`. Ambos podem ser tratados com `se_falhar`.

---

## Núcleo

As funções e os valores do núcleo são importadas automaticamente e estão sempre disponíveis, sem necessidade de `importe`. As funções fornecidas pela linguagem são chamadas de **funções nativas**.

### `escreva(valor, …)` / `escreva()`

Mostra os argumentos separados por espaços e termina com uma quebra de linha. Sem argumentos, escreve uma linha vazia. Retorna `nada`. Um número sozinho segue o formato ativo (`2,5`); dentro de uma lista, mapa ou par, aparece como no código (`[1.5, 2]`).

```text
escreva("Olá", 10)
escreva()
```

### `escreva_erro(valor, …)`

Funciona como `escreva`, mas usa a saída de erro (`stderr`). No terminal, o redirecionamento `> arquivo` captura apenas a saída padrão, sem incluir essas mensagens.

```text
escreva_erro("nome vazio")
```

### `saia()` / `saia(codigo)`

Encerra o programa com o código informado. Sem argumento, usa o código 0. Os comandos após a chamada não são executados. No REPL do terminal, a linha `sair` (sem `()`) sai do interpretador; `saia()` encerra o processo. No painel REPL do expressa-aula, `saia()` encerra apenas a sessão; **Reiniciar** começa outra. O sistema operacional considera o código de saída 0 como **sucesso** e qualquer outro como **erro**.

```text
se nome == "" { saia(1) }
```

### `falhe(descricao)`

Gera um erro de execução com a descrição informada. A pilha de chamadas indica o local que chamou `falhe`, sem adicionar uma entrada para a própria função. O erro pode ser tratado com `se_falhar`.

```text
se n < 0 { falhe("negativo") }
```

### `garanta(condicao)`

Recebe uma condição cujo valor deve ser `verdadeiro` ou `falso`. Se ela for falsa, gera um erro com a mensagem `condição incorreta` seguida do texto da condição. O erro pode ser tratado com `se_falhar`.

```text
garanta(n > 0)
```

### `leia()` / `leia(mensagem)`

Lê uma linha digitada no teclado e retorna o texto sem a quebra de linha final. A mensagem opcional é mostrada antes da leitura, sem acrescentar uma quebra de linha. Se a entrada terminar sem fornecer uma linha, gera um erro que pode ser tratado com `se_falhar`.

```text
nome = leia("Seu nome: ")
```

### `leia_linhas()`

Lê a entrada padrão até o fim e retorna uma lista de textos, um por linha. Se a entrada estiver vazia, retorna `[]`. No expressa-aula, essa função não está disponível e gera um erro.

```text
para linha em leia_linhas() { escreva(linha) }
```

### `argumentos`

É a lista dos argumentos passados após o nome do arquivo `.lep` na linha de comando. O primeiro fica em `argumentos[0]`. No REPL, a lista é vazia (`[]`). `argumentos` é um valor, não uma função, e não pode ser reatribuído.

```text
escreva(argumentos[0])
```

### `tipo(valor)`

Retorna o nome do tipo como texto: `numero`, `intervalo`, `texto`, `bool`, `lista`, `par`, `mapa`, `conjunto`, `matriz`, `imagem`, `funcao`, `modulo`, `processo` ou `nada`. Os nomes são os mesmos usados nas mensagens de erro, sem acentos. Tanto funções nativas quanto funções definidas no programa têm o tipo `"funcao"`.

```text
se tipo(x) == "numero" { x * 2 } senão { x }
x.tipo()
```

### `tipos()`

Retorna uma lista de textos com os nomes de todos os tipos que `tipo()` pode retornar.

```text
tipos()
tipos() contem "mapa"
```

### `numero(texto)` / `número(texto)`

Converte um texto em número, seguindo a configuração de `formato`: `"3.14"` no formato `"en"`, por exemplo, ou `"3,14"` no formato `"pt"`. Aceita espaços no início e no fim e `_` como separador entre dígitos. Se o argumento já for um número, retorna o próprio valor.

A leitura é estrita. No formato `"pt"`, a vírgula separa os decimais e o ponto só separa milhares em grupos de três: `numero("1.500")` é 1500, e `numero("2.5")` gera um erro que sugere escrever `2,5` ou usar `define_opcao(:formato_numero, :en)`. O formato `"en"` é o espelho: `numero("1,500")` é 1500 e `numero("2,5")` gera erro.

```text
idade = numero(leia("Idade: ")) se_falhar { 0 }
```

### `define_opcao(opcao, valor)`

Configura a execução inteira. Também aceita a grafia `define_opção`.

| Opção | Valores | Padrão |
| --- | --- | --- |
| `:tipo_numero` | `:decimal` ou `:float` | `:decimal` |
| `:formato_numero` | `:pt` ou `:en` | `:pt` |
| `:fechar_janelas` | `verdadeiro` ou `falso` | `falso` |

As duas configurações numéricas são independentes. `:tipo_numero` escolhe a aritmética; `:formato_numero` escolhe a apresentação dos números e a interpretação de textos por `numero()`. Os literais do código continuam usando ponto decimal. As opções de linha de comando `--numeros en` e `--aritmetica float` continuam disponíveis para escolher os valores iniciais.

Com `:fechar_janelas` ativa, as janelas gráficas e os gráficos cartesianos desta execução são fechados ao terminar, inclusive em caso de erro, `saia()` ou interrupção pelo botão Parar. A opção também abrange janelas abertas antes da chamada. Uma chamada posterior com `falso` desativa esse fechamento. No REPL, a execução corresponde à sessão inteira.

A função devolve `nada`. Nomes de opções ou valores inválidos produzem erro e não alteram a configuração anterior. Ela substitui a antiga função `formato`, que foi removida.

```text
define_opcao(:formato_numero, :en)
define_opcao(:tipo_numero, :float)
define_opcao(:fechar_janelas, verdadeiro)
```

### `formate(modelo, valor, …)`

Substitui os marcadores do modelo pelos valores informados. `{}` usa o próximo valor; `{0}` usa o primeiro valor. Para alinhar em um espaço de largura `n`, use `{:<n}` à esquerda, `{:>n}` à direita ou `{:^n}` ao centro. Use `{{` e `}}` para escrever as próprias chaves. A largura deve estar entre 0 e 10000; valores maiores geram erro.

```text
formate("{:<8} {:>5}", "Ana", 10)
```

Um mapa pode definir `:_como_texto` (função de um argumento que devolve texto). `formate`, `escreva` e o `+` com texto chamam essa função. Sem o campo, o mapa continua `mapa{…}`. Você define essa função no programa, por exemplo com `fracao_texto = função(fr) { … }`, e a associa ao mapa com `fr:_como_texto = fracao_texto`.

Ex:

```expressa
fracao_texto = função(t) { formate("{}/{}", t:num, t:div) }
fracao = mapa([
    :num -> 1,
    :div -> 2,
    :_como_texto -> fracao_texto,
])
escreva(fracao)
// Retorna 1/2
```

### `como_texto(valor)`

Retorna o texto que `escreva` mostraria para o valor: números sozinhos seguem o formato ativo, e um mapa com `:_como_texto` usa essa função.

```text
como_texto(2.5)      // "2,5"
como_texto([2.5])    // "[2.5]"
```

### `avalie(texto)`

Executa o texto como código Expressa no escopo atual e retorna o último valor. Erros de sintaxe ou de execução podem ser tratados com `se_falhar`.

```text
avalie("2 + 3 * 4")     // 14
```

### `modulos()` / `módulos()`

Imprime os módulos disponíveis, em ordem alfabética, e devolve `nada`. Inclui `nucleo`, os módulos nativos disponíveis e arquivos já importados na sessão. Cada arquivo aparece uma vez, mesmo quando importado por vários apelidos. O nome é o primeiro caminho usado na importação; caminhos absolutos distinguem nomes em conflito.

`modulos(:dados)` devolve uma lista de mapas com `:nome` (texto) e `:funcoes` (a lista estruturada de funções do módulo), sem imprimir.

### `funcoes()` / `funções()`

Imprime as funções públicas com argumentos e documentação completa, em ordem alfabética, e devolve `nada`. Sem argumento, ou com `nada`, consulta o escopo atual. O alvo pode ser o nome de um módulo nativo, `"nucleo"`, o caminho de um arquivo já importado, um apelido ou o próprio módulo. Valores que não são funções e nomes iniciados por `_` ficam de fora.

Com `:dados` no segundo argumento, devolve uma lista de mapas com `:nome` (texto), `:args` (lista de textos) e `:doc` (documentação completa; `""` se não houver), sem imprimir.

```text
modulos()
funcoes("arquivo")
f = funcoes(nada, :dados)
m = modulos(:dados)
biblioteca = importe "./minha_biblioteca"
funcoes(biblioteca)
```

### `ajuda(alvo)`

Mostra a ajuda de uma função. O argumento pode ser um nome como `"escreva"` ou `"matriz::zeros"`, ou a própria função.

```text
ajuda("escreva")
```

### `mapa()` / `mapa(pares)`

Cria um mapa vazio ou um mapa a partir de uma lista de pares. Chaves duplicadas geram erro. A ordem de inserção não é preservada; mapas são iguais quando têm as mesmas chaves com valores iguais. A impressão ordena as chaves: primeiro textos, depois inteiros e valores lógicos, cada grupo em ordem crescente.

```text
pessoa = mapa([:nome -> "Ana", :idade -> 25])
pessoa:nome = "Bia"
pessoa["cidade"] = "Fortaleza"
pessoa = pessoa.remova("idade")
pessoa += :idade -> 26
para p em pessoa { escreva(p:chave, p:valor) }
```

### `chaves_de(mapa)` / `valores_de(mapa)`

Retornam a lista das chaves e a lista dos valores do mapa, na ordem da impressão.

```text
chaves_de(mapa([:b -> 2, :a -> 1]))     // ["a", "b"]
valores_de(mapa([:b -> 2, :a -> 1]))    // [1, 2]
```

### `conjunto()` / `conjunto(xs)`

Cria um conjunto de valores únicos, que podem ser textos, números inteiros ou valores lógicos. Valores repetidos são ignorados. A iteração não tem ordem garantida. A impressão ordena os elementos como as chaves de mapa; a igualdade depende apenas dos elementos.

```text
s = conjunto([:ana, :bia])
s += :carlos
s = s.remova(:bia)
```

### `tamanho(valor)`

Retorna a quantidade de caracteres de um texto, de itens de uma lista ou conjunto, de pares de um mapa, de linhas de uma matriz ou de inteiros gerados por um intervalo (`tamanho(0..3)` é 3). Em textos, conta caracteres Unicode, não bytes. Um intervalo sem teto, como `0..`, gera erro.

```text
tamanho("olá")    // 3
```

### `primeiro(lista)` / `ultimo(lista)`

Retornam, respectivamente, o primeiro e o último item da lista. Em um texto, retornam o primeiro e o último caractere (`primeiro("olá")` é `"o"`). Também aceitam um intervalo: `primeiro(3..5)` é 3 e `ultimo(3..5)` é 4, o último número gerado. Geram erro se não houver elementos; `ultimo` também gera erro em um intervalo sem teto. `ultimo` também se escreve `último`.

```text
primeiro([10, 20])
```

### `eh_infinito(intervalo)` / `é_infinito(intervalo)`

Retorna `verdadeiro` se o intervalo não tem teto, como `0..`, e `falso` caso contrário. Outro tipo de valor gera erro.

```text
eh_infinito(0..)     // verdadeiro
eh_infinito(0..3)    // falso
```

### `maiuscula(texto)` / `minuscula(texto)`

Retornam uma cópia do texto em letras maiúsculas ou minúsculas, respectivamente. Também se escrevem `maiúscula` e `minúscula`.

```text
maiuscula("olá")
```

### `sem_acento(texto)`

Retorna uma cópia sem acentos nem cedilha, como `ã` e `á` → `a`, e `ç` → `c`. Preserva a distinção entre maiúsculas e minúsculas.

```text
sem_acento("São Paulo")
```

### `remova`

`remova(lista, indice)` /
`remova(lista, intervalo)` /
`remova(mapa, chave)` /
`remova(conjunto, valor)`

Retorna uma cópia sem o item do índice indicado, sem os itens do intervalo, sem a chave do mapa ou sem o valor do conjunto. O índice final do intervalo não é incluído; use `..=` para incluí-lo. O intervalo precisa ter teto: `remova(xs, 1..)` gera erro.

```text
xs = xs.remova(2)
xs = xs.remova(2..4)    // tira os índices 2 e 3
xs = xs.remova(2..=3)   // o mesmo
```

### `substitua(texto, antigo, novo)`

Retorna uma cópia do texto com todas as ocorrências de `antigo` substituídas por `novo`.

```text
substitua("aa", "a", "b")    // "bb"
```

### `procure(texto, trecho)` / `procure(lista, item)`

Retorna o índice da primeira ocorrência, contando a partir de zero, ou `nada` se não encontrar. Na busca em texto, um trecho vazio (`""`) gera erro.

```text
procure("onde", "n")    // 1
procure("onde", "x")    // nada
```

### `separe(texto, separador)`

Divide o texto em uma lista de partes usando o separador indicado. Com o separador vazio (`""`), retorna um item por caractere.

```text
separe("a,b", ",")    // ["a", "b"]
```

### `junte(lista, separador)`

Converte os itens da lista em texto e os une, colocando o separador entre eles.

```text
junte(["a", "b"], "-")    // "a-b"
```

### `limpe(texto)`

Retorna uma cópia do texto sem os espaços do início e do fim.

```text
limpe("  x  ")    // "x"
```

### `ordene(lista)` / `ordene(lista, chave)`

Retorna uma cópia da lista em ordem crescente. Os itens devem ser todos números ou todos textos. Para ordenar por uma característica dos itens, forneça uma função que calcule a chave de ordenação: `ordene(pessoas, função(p) { p:idade })`.

### `comeca_com(texto, prefixo)` / `começa_com` · `termina_com(texto, sufixo)`

Retornam `verdadeiro` se o texto começa com o prefixo ou termina com o sufixo informado, respectivamente; caso contrário, retornam `falso`.

### Listas e sub-listas

```text
xs = [10, 20, 30, 40]
xs[0]           // 10
xs[1..3]        // [20, 30]
xs[1..]         // até o fim
xs + [50]       // nova lista [10, 20, 30, 40, 50]; xs não muda
xs += [50]      // xs passa a ser [10, 20, 30, 40, 50]
"abc"[0..2]     // "ab"
"abc"[1..]      // "bc"
0..3            // intervalo 0, 1, 2; 0..=2 é o mesmo
t = "abc"
t[1] = "x"      // substitui um caractere; t passa a ser "axc"
```

Nativas do núcleo não podem ser reatribuídas (`escreva = 1` é erro).

### `clone(valor)`

Retorna uma cópia profunda de listas, mapas, conjuntos, pares, matrizes e imagens, preservando ciclos e referências compartilhadas internas. `a = b` e `xs + []` não fazem isso: a primeira compartilha o mesmo objeto, e a segunda copia apenas o primeiro nível. Funções que usam nomes de fora, módulos e processos não podem ser clonados. Veja o [contrato completo](clone.md).

```text
b = clone([[1, 2]])
b[0][0] = 9           // a lista original não muda
```

---

## `importe "tela"`

Oferece funções para controlar o terminal e o painel de saída do expressa-aula.

### `limpe_tela()`

Limpa a tela e coloca o cursor no canto superior esquerdo. No REPL, a palavra `limpe_tela` ou `cls`, sem parênteses, também limpa a tela. Limpar a cada quadro de uma animação pode causar piscadas; use `posicao_cursor(0, 0, falso)` para voltar ao início sem apagar o conteúdo.

### `posicao_cursor(linha, coluna, mostre)`

Move o cursor sem apagar, com linha e coluna inteiras a partir de zero.
`mostre` controla o envio imediato da saída ao terminal (*flush*). O valor padrão é `verdadeiro`, e o argumento pode ser omitido.
Em animações, use `posicao_cursor(0, 0, falso)` para enviar o movimento junto
com a próxima escrita.

### Consultar o cursor e repetir uma entrada

Com `importe "tela"`, `posicao_cursor()` devolve um mapa com `:linha` e
`:coluna`, começando em zero. `posicao_cursor(pos)` volta à posição guardada.
`apague_linha()` limpa a linha atual inteira e coloca o cursor no início dela.

```lep
importe "tela"

escreva("1 - Continuar | 2 - Sair")
pos = posicao_cursor()
enquanto verdadeiro {
    posicao_cursor(pos)
    apague_linha()
    opcao = leia("Opção: ")
    se opcao == "1" ou opcao == "2" {
        apague_linha()
        pare
    }
    apague_linha()
    escreva("Opção inválida. Escolha 1 ou 2.")
}
```

O aviso permanece na linha de baixo enquanto a entrada é refeita na linha de
cima. O expressa-aula permite digitar nessa posição sem apagar o aviso. No terminal,
a consulta exige suporte à posição do cursor; com a saída redirecionada para um arquivo ou outro programa, ela falha.
No expressa-aula, a numeração conta quebras explícitas, não as quebras visuais de linhas
longas. Rolagem pode invalidar uma posição guardada.

### `eh_terminal()` / `é_terminal()`

Retorna `verdadeiro` se a entrada vem de um terminal interativo. Retorna `falso` quando ela vem de outro programa por um *pipe*, de um arquivo ou do expressa-aula.

### `pinte(texto, frente)` / `pinte(texto, frente, fundo)`

Retorna o texto com códigos ANSI para definir a cor das letras e, opcionalmente, do fundo. Ao final, restaura a formatação padrão. As cores são `normal`, `preto`, `vermelho`, `verde`, `amarelo`, `azul`, `magenta`, `ciano` e `branco`. `:verde` equivale ao texto `"verde"`. Use `normal` para a cor padrão das letras ou do fundo, conforme o argumento.

```text
"HP".pinte(:verde)
"HP".pinte(:branco, :vermelho)
```

### `fundo(texto, cor)`

Aplica uma cor de fundo ao texto, mantendo a cor padrão das letras.

### `negrito(texto)`

Retorna o texto com códigos ANSI de negrito e restaura a formatação padrão ao final.

### `tamanho_tela()`

Retorna o tamanho da tela em caracteres: um mapa com `:linhas` e `:colunas`. Sem um terminal interativo, usa 24 linhas e 80 colunas. O painel do expressa-aula também informa esses valores fixos.

```text
tamanho_tela():colunas    // 80 sem terminal
```

### `quadro(texto)`

Move o cursor para o canto superior esquerdo e escreve o texto. Apaga o conteúdo que sobrar à direita de cada linha e abaixo do texto. Use `nova_linha` para separar as linhas.

### `nova_linha`

Valor, não função: `\n` no Unix e no expressa-aula; `\r\n` no Windows.

```text
tela += linha + nova_linha
```

### `bloco(linha, coluna)`

Cria um mapa com os campos `:linha` e `:coluna`, contando a partir de zero. Ele guarda a posição usada por `escreva_em`; a criação do bloco não desenha nada.

### `escreva_em(bloco, texto)`

Escreve o texto na posição indicada pelo bloco, avança a linha do bloco e retorna o mesmo mapa. Você também pode chamar com ponto: `b.escreva_em("Q W E")`. O texto não pode conter quebras de linha (`\n`). O conteúdo à esquerda na mesma linha é preservado.

```text
b = bloco(0, 27)
b.escreva_em("Q W E R T")
```

---

## `importe "cartesiano"`

Desenha gráficos de funções no plano cartesiano, com o eixo y crescendo para cima. Um **documento** reúne os gráficos exibidos em uma janela. Chamadas de `mostre` para o mesmo documento atualizam e trazem essa janela para a frente.

```text
importe "mat"
cart = importe "cartesiano"

c = cart::documento()
g = cart::grafico()
g.cart::eixos(-2 * pi, 2 * pi, -1.5, 1.5)
g.cart::plote(função(x) { seno(x) }, :azul)
g.cart::plote(função(x) { cosseno(x) }, :vermelho)
c.cart::adicione(g)
c.cart::mostre()
```

`adicione` guarda uma cópia do gráfico no documento. Alterações posteriores no gráfico original não mudam essa cópia. Outra chamada de `mostre` para o mesmo documento atualiza a janela existente. Documentos diferentes usam janelas diferentes.

As cores são as mesmas do módulo `tela`. `tipo(g)` retorna `"mapa"`. No expressa-aula, o resultado aparece no painel **Gráficos**; use **Destacar** para abrir uma janela separada. No terminal, `expressa programa.lep` abre a janela automaticamente.

### `documento()`

Cria um documento vazio, representado por um mapa. A janela só é aberta quando você chama `mostre`.

### `grafico()` / `gráfico()`

Cria um gráfico vazio. Configure os limites com `eixos` e acrescente curvas com `plote`.

### `eixos(grafico, xmin, xmax, ymin, ymax)`

Define os limites dos eixos. É necessário que `xmin < xmax` e `ymin < ymax`. Configure os eixos antes de chamar `plote`; caso contrário, ocorre um erro.

### `plote(grafico, f, cor)`

Adiciona uma curva calculada pela função `f`, que recebe x e retorna y. O gráfico usa aproximadamente 400 amostras. Valores infinitos de y interrompem o traçado naquele ponto.

### `adicione(documento, grafico)`

Adiciona uma cópia do gráfico ao documento e retorna o documento. O gráfico precisa ter pelo menos uma curva.

### `mostre(documento)`

Abre a janela do documento ou atualiza a janela existente. A execução continua sem esperar que a janela seja fechada. Um documento vazio gera erro, que pode ser tratado com `se_falhar`.

---

## `importe "tela_grafica"`

Cria desenhos em uma tela de pixels. Cada tela gráfica é representada por um mapa e pode ser exibida em sua própria janela. Use funções como `linha`, `circulo`, `ponto`, `retangulo` e `imagem` para desenhar; depois, chame `mostre` para exibir ou `salve` para gravar em um arquivo.

As cores são as mesmas do módulo `tela`, além de `transparente`. `fundo(g, cor)` define a cor de fundo. `como_imagem(g)` converte o desenho em um valor do tipo `imagem`, preservando as áreas transparentes.

```
tela_grafica = importe "tela_grafica"
g = tela_grafica::nova(320, 200)
g.tela_grafica::cor(:azul)
g.tela_grafica::linha(10, 10, 310, 190)
g.tela_grafica::texto(10, 10, "casa")
g.tela_grafica::mostre()
g.tela_grafica::salve("casa.png")
```

`(0, 0)` é o canto superior esquerdo, e o eixo y cresce para baixo. Cada chamada de `nova()` cria uma tela independente. `mostre` abre a janela na primeira chamada e atualiza a mesma janela nas seguintes.

`g.tela_grafica::feche()` fecha apenas a janela de `g` e solicita interromper os laços associados a essa tela. Não apaga o desenho; `mostre()` pode abrir a janela novamente. O fechamento é assíncrono e pode ser solicitado mesmo antes de a janela abrir ou depois de já ter sido fechada. `l.tela_grafica::pare()` apenas interrompe o laço e mantém o último quadro visível. Em um jogo, use `define_opcao(:fechar_janelas, verdadeiro)` para fechar as janelas ao terminar, ou chame `feche()` quando precisar fechar uma tela explicitamente.

`nova(160, 144, 4)` cria uma área de desenho de 160 × 144 pixels e a exibe em 640 × 576 pixels: cada pixel do desenho ocupa um quadrado de 4 × 4 pixels na janela.

`texto(g, x, y, t)` escreve a partir de `(x, y)`, usando a cor atual. O tamanho padrão dos caracteres é de 8 pixels; `texto(g, x, y, t, 16)` dobra esse tamanho. `pixel(g, x, y)` retorna a cor do ponto indicado.

`evento(g)` consulta o teclado e os acontecimentos pendentes da janela **sem esperar uma interação**. A função recebe apenas a tela gráfica como argumento. Chame `mostre()` antes e consulte `evento()` uma vez por quadro. O resultado é um mapa com campos como `:teclas`, `:apertadas`, `:soltas`, `:eventos` e `:sair`.

### Laço gráfico

Para jogos e animações, prefira o laço gráfico em vez de um `enquanto` com `evento()` e `durma()`. `laco_grafico(g)` (também `laço_gráfico`) cria o laço; `execute(l, config)` o inicia e espera que ele termine. `config` é um mapa com exatamente três opções: `:fps` (quadros por segundo), `:eventos` (função que recebe `ev` e `dt`) e `:desenho` (função que recebe a tela). A cada quadro, o laço lê a entrada, chama `eventos(ev, dt)`, apaga o desenho, chama `desenho(g)` e mostra o resultado. `ev` é o mesmo mapa devolvido por `evento()`; `dt` é o tempo, em segundos (ou fração de segundos), desde a chamada anterior. `pare_laco(l)` (também `pare_laço`, ou `l.tela_grafica::pare()`) encerra o laço depois do quadro atual. Fechar a janela também encerra. Dentro das funções do laço, não chame `evento()`, `mostre()` nem `durma()`.

```text
tela_grafica = importe "tela_grafica"
g = tela_grafica::nova(160, 144, 4)
l = g.tela_grafica::laco_grafico()
x = mapa([:valor -> 0])

eventos = função(ev, dt) {
    se ev:apertadas contem :escape { l.tela_grafica::pare() }
    se ev:teclas contem :direita { x:valor += 60 * dt }
}
desenhe = função(tela) {
    tela.tela_grafica::retangulo(x:valor, 60, x:valor + 8, 68)
}
l.tela_grafica::execute(mapa([:fps -> 30, :eventos -> eventos, :desenho -> desenhe]))
```

Como as funções do laço não podem reatribuir nomes de fora, guarde o estado que muda entre quadros em um mapa, como `x` acima.

No expressa-aula, a tela gráfica aparece em uma janela separada do painel **Gráficos**. Para detalhes técnicos da comunicação com o visualizador, consulte [Tela gráfica](tela_grafica.md).

## `importe "imagem"`

Oferece funções para carregar, salvar e transformar imagens PNG em memória: `carregue`, `salve`, `rotacione`, `redimensione`, `largura` e `altura`. O tipo do valor retornado por `carregue` é `"imagem"`.

Para desenhar com `tela_grafica::imagem`, você pode passar um valor de imagem, o caminho de um arquivo (carregado uma vez) ou outra tela gráfica. O exemplo abaixo usa a tela `g` criada na seção anterior e um arquivo `tartaruga.png` disponível na pasta do programa.

```
img = importe "imagem"
figura = img::carregue("tartaruga.png")
g.tela_grafica::imagem(figura, 160, 100, 90)
figura.img::salve("copia.png")
```

---

## `importe "mat"`

### `raiz(n)`

Retorna a raiz quadrada de `n`. Um número negativo gera erro, que pode ser tratado com `se_falhar`.

### `aleatorio(min, max)` / `aleatório`

Retorna um número inteiro aleatório entre `min` e `max`, incluindo os dois limites. Sem uma chamada a `semente`, cada execução do programa gera uma sequência diferente.

### `semente(n)`

Define a semente usada por `aleatorio`. A mesma semente produz a mesma sequência, o que permite repetir testes.

### `pi`

Valor, não função: `pi`, não `pi()`.

```text
area = pi * r * r
```

### `seno` / `sen` · `cosseno` / `cos` · `tangente`

Calculam as funções trigonométricas com o ângulo em **radianos**. Para usar graus, converta o valor antes: `seno(radianos(30))`.

### `arcosseno` `arcocosseno` `arcotangente` `arcotangente2(y, x)`

Calculam as funções trigonométricas inversas e retornam ângulos em radianos. Em `arcosseno` e `arcocosseno`, o argumento deve estar entre -1 e 1.

### `radianos(graus)` / `graus(radianos)`

Convertem ângulos entre graus e radianos. Por exemplo, `graus(pi)` retorna 180.

### `exp(x)` · `log(x)` · `log10(x)`

`exp(x)` calcula e elevado a x. `log(x)` calcula o logaritmo natural e `log10(x)`, o logaritmo na base 10. Por exemplo, `log10(1000)` retorna 3. Para `log` e `log10`, x deve ser maior que zero.

### `potencia(base, exp)` / `potência(base, exp)`

Eleva a base ao expoente informado, que pode ser fracionário. Zero elevado a zero vale 1.

### `piso(x)` · `teto(x)` · `arredonde(x)`

`piso` retorna o maior inteiro menor ou igual ao valor; `teto` retorna o menor inteiro maior ou igual ao valor; `arredonde` retorna o inteiro mais próximo. Quando o valor fica exatamente entre dois inteiros, `arredonde` escolhe o mais distante de zero: `arredonde(1.5)` retorna 2 e `arredonde(-1.5)` retorna -2.

---

## `importe "sistema"`

As funções deste módulo exigem `importe "sistema"`.

### `durma(segundos)`

Pausa a execução pelo número de segundos informado. Aceita valores fracionários, como `durma(0.5)` para meio segundo. `durma(0)` não espera.

```text
importe "sistema"
durma(1)
```

### `ambiente(nome)`

Retorna o valor de uma variável de ambiente. Se ela não existir, gera um erro. Você pode fornecer um valor alternativo com `ambiente("EDITOR") se_falhar { "vim" }`.

---

## `importe "crono"`

O módulo `crono` oferece funções para trabalhar com datas e horários.

- `agora()` retorna um mapa com `:ano`, `:mes`, `:dia`, `:hora`, `:minuto`, `:segundo`, `:dia_da_semana` (1 corresponde a domingo) e `:epoch`.
- `hoje()` retorna a data de hoje à meia-noite.
- `data(2024, 2, 29)` constrói uma data; você também pode informar o horário.
- `adicione(d, n, unidade)` soma uma quantidade à data. As unidades são `:dias`, `:meses`, `:anos`, `:horas`, `:minutos` e `:segundos`. Use um número negativo para subtrair.
- `dias_entre(a, b)` conta os dias de calendário entre duas datas.
- `eh_bissexto(2024)` informa se o ano é bissexto.
- `formate_data(agora(), "%Y-%m-%d %H:%M:%S")` retorna a data e o horário no formato indicado.

---

## `importe "matriz"`

Uma matriz organiza números em linhas e colunas. `A[0, 1]` acessa a linha 0 e a coluna 1. `A[0]` retorna a primeira linha como lista.

Para gráficos, `A.xy(x, y)` lê `A[y, x]` e `A.xy(x, y, valor)` altera essa célula, devolvendo a mesma matriz para encadear chamadas. Ambas usam coordenadas inteiras em base zero e os mesmos limites do acesso por índices. A escrita exige um número e é visível por outras referências à matriz.

```text
A = matriz([[1, 2, 3], [4, 5, 6]])
A[0, 1]          // 2
A.xy(1, 0)       // 2: x é coluna, y é linha
A.xy(1, 0, 9)  // altera A[0, 1]; devolve A
transposta(A)
det(matriz([[1, 2], [3, 4]]))  // determinante; aceita matrizes 1×1, 2×2 ou 3×3
identidade(3)
zeros(2, 5)
uns(3)
cheia(2, 3, 7)
linhas(A)        // 2
colunas(A)       // 3
tamanho(A)       // também as linhas
A + B
3 * A
A * B            // produto de matrizes
```

---

## `importe "arquivo"`

Os caminhos relativos partem da pasta do arquivo `.lep`. No expressa-aula, partem da pasta do projeto. Use `~` para indicar sua pasta pessoal, como em `~/pasta/arquivo.txt`; esses caminhos também permitem gravar na pasta pessoal.

### `leia_arquivo(caminho)`

Lê o arquivo e retorna uma lista de textos, um por linha, sem as quebras de linha. Se o arquivo estiver vazio, retorna `[]`. Falhas de leitura geram erro e podem ser tratadas com `se_falhar`.

```text
linhas = leia_arquivo("dados.txt") se_falhar { [] }
```

### `salve_arquivo(caminho, linhas)`

Grava os itens no arquivo, um por linha, substituindo o conteúdo anterior. Cria as pastas necessárias se elas não existirem.

### `adicione_arquivo(caminho, linhas)`

Acrescenta as linhas ao fim do arquivo. Cria o arquivo se ele não existir.

### `existe(caminho)` · `eh_pasta(caminho)` / `é_pasta`

`existe` retorna `verdadeiro` se o caminho existir. `eh_pasta` retorna `verdadeiro` se o caminho indicar uma pasta. Caso contrário, retornam `falso`.

### `liste(pasta)` / `liste(pasta, padrao)`

Retorna os nomes dos arquivos e das pastas em ordem, incluindo os ocultos. O padrão opcional filtra os nomes: `*` corresponde a qualquer trecho e `?` a um caractere. Por exemplo, `liste("fotos", "*.png")` lista os nomes com extensão `.png`.

### `crie_pasta(caminho)` · `apague_arquivo(caminho)` · `apague_pasta(caminho)`

`crie_pasta` cria a pasta e as pastas intermediárias que faltarem. `apague_arquivo` remove um arquivo. `apague_pasta` remove a pasta e todo o conteúdo dela; a função recusa apagar a raiz (`/`) e a pasta pessoal.

### `mova(origem, destino)` · `copie(origem, destino)`

Movem ou copiam a origem para o destino. Se o destino for uma pasta, o arquivo é colocado dentro dela. `copie` aceita apenas arquivos.

### `junte_caminho(parte, …)` · `nome_do_arquivo` · `pasta_de` · `extensao` · `pasta_atual()`

`junte_caminho`, `nome_do_arquivo`, `pasta_de` e `extensao` montam ou consultam o texto de um caminho, sem acessar seu conteúdo. `pasta_atual()` é a pasta do terminal; os caminhos relativos das outras funções partem da pasta do `.lep`:

```text
entrada = junte_caminho(pasta_atual(), argumentos[0])
```

### `leia_bytes()` / `leia_bytes(caminho)` · `texto_utf8(bytes)` / `texto_utf8(bytes, modo)`

`leia_bytes` lê o arquivo sem alterar as quebras de linha e retorna uma lista de números de 0 a 255. Sem caminho, lê a entrada padrão até o fim; no expressa-aula, informe um caminho. `texto_utf8` converte essa lista em texto. O modo padrão, `:estrito`, gera erro com bytes que não formam UTF-8; `:substitua` troca esses bytes por `�`, e `:ignore` os descarta.

```text
texto = texto_utf8(leia_bytes("dados.txt"), :substitua)
```

### `leia_csv(caminho)` / `salve_csv(caminho, dados)`

Leem e gravam um CSV simples, separado por vírgulas e sem tratamento de campos entre aspas. `leia_csv` retorna uma lista de listas de **texto**. `salve_csv` substitui o conteúdo do arquivo.

```text
dados = leia_csv("pessoas.csv")
salve_csv("saida.csv", [["Ana", 25]])
```

---

## `importe "processo"`

Executa outros programas pela linha de comando. Esse módulo não está disponível no expressa-aula; usá-lo ali gera um erro.

O primeiro argumento é um comando, escrito como texto (`"ls -l"`) ou como uma lista de argumentos (`["wc", "-l"]`). No texto, `|` encadeia comandos. Depois do primeiro comando, você pode passar outras etapas: a saída de uma etapa alimenta a seguinte.

Uma etapa também pode ser uma função, que recebe uma linha por chamada. Se retornar `nada`, a linha é descartada; se retornar texto, esse texto segue para a próxima etapa; se retornar uma lista, cada item segue como uma linha. Para passar argumentos adicionais à função, use uma lista como `[filtro, ".txt"]`, que chama `filtro(linha, ".txt")`.

O comando não passa por um interpretador de comandos (*shell*). Operadores como `*`, `>`, `$` e `&&` geram erro nessa forma de chamada. Se precisar de recursos do shell, chame-o explicitamente, por exemplo com `["sh", "-c", "…"]`.

O último argumento pode ser um mapa de opções:

| Opção | Efeito |
|---|---|
| `:pasta` | define a pasta de execução do comando |
| `:entrada` | fornece um texto ou uma lista de linhas como entrada do primeiro comando |
| `:ambiente` | fornece um mapa de variáveis de ambiente |
| `:tempo` | define o tempo-limite em segundos |
| `:exija -> verdadeiro` | gera um erro se o código de saída for diferente de zero |
| `:terminal -> verdadeiro` | conecta o comando ao teclado e à tela; disponível em `execute`, apenas com etapas que sejam comandos |

### `execute(comando, etapa, …)`

Executa as etapas e espera que terminem. Retorna um mapa com `:codigo` (código de saída), `:ok` (sucesso ou falha), `:saida` (linhas da saída padrão) e `:erro` (linhas da saída de erro). Por padrão, um código diferente de zero não gera um erro da linguagem; consulte `:ok` ou use a opção `:exija`.

```text
filtro = funcao(linha, oque) { se linha contem oque { linha } }
n = execute("ls", [filtro, ".txt"], ["wc", "-l"]):saida[0]
r = execute("git status --short")
se nao r:ok { escreva_erro(r:erro) }
```

```text
execute(["vim", "notas.txt"], mapa([:terminal -> verdadeiro]))
execute("cargo build", mapa([:terminal -> verdadeiro, :exija -> verdadeiro]))
```

### `linhas_de(comando, etapa, …)`

Inicia o comando e retorna um processo que pode ser percorrido com `para … em`. Cada linha fica disponível à medida que chega. Sair do laço encerra o comando. A saída de erro é mostrada na tela.

```text
para linha em linhas_de("tail -f app.log") {
    se linha contem "xxx" { escreva(linha) }
    se linha contem "FIM" { pare }
}
```

---

## REPL e depurador

No expressa-aula, **Ver → REPL** abre um painel em que as variáveis permanecem entre comandos
e a saída aparece durante a execução. **Parar** interrompe a avaliação atual;
**Reiniciar** cancela a sessão e esquece as variáveis. **Limpar** apaga apenas a saída.
O painel mantém até 5000 linhas e 262144 caracteres, descartando o texto mais antigo.
Resultados longos são abreviados com um aviso. Fechar a IDE encerra a sessão.

No terminal:

```text
expressa
modulos()
funcoes()
funcoes("matriz")
ajuda("escreva")
ajuda("matriz::zeros")
limpe_tela      // limpa a tela do REPL (também cls)
sair
```

```text
expressa debug prog.lep
continuar (c)   proximo (n)   entrar (s)   sair (o)
vars (v)   pilha (k)   ponto N   terminar (q)
```

No expressa-aula: **F6** inicia a depuração; **F5** continua quando pausado no depurador
(ou executa normalmente quando não há uma execução em andamento); F9 ponto de
parada; F10 próximo; F11 entrar.

Durante uma pausa, o código-fonte da versão em execução aparece em uma
visualização somente para leitura. **Parar** fecha essa visualização imediatamente
e retorna ao editor; o término da execução também a fecha. Erros de **F5** aparecem no console e apontam para o código editável quando
ele corresponde à versão executada. **Ver → Ver código executado** permite consultar
essa versão explicitamente, inclusive quando você já alterou o texto atual.

---

## Números: texto e aritmética

Literais no código: sempre ponto (`3.14`).

`define_opcao(:formato_numero, :pt)` e `define_opcao(:formato_numero, :en)` configuram a **representação textual** dos números, tanto na exibição quanto na conversão de texto com `numero`.

`define_opcao(:tipo_numero, :decimal)`, o padrão, faz a aritmética em base 10. `define_opcao(:tipo_numero, :float)` usa números de ponto flutuante IEEE-754 nas operações `+`, `-`, `*`, `/` e `%`. Essa escolha é independente do formato textual `pt` ou `en`.

Os módulos `mat`, `matriz`, `cartesiano` e `tela_grafica` calculam internamente com números de ponto flutuante binários (as células de matriz são sempre binárias). No modo decimal, um resultado que é só ruído binário em torno de um número com até 15 algarismos significativos volta como esse número: `seno(radianos(30))` é `0,5`, `cosseno(radianos(60))` é `0,5` e `3 * matriz([[0.1]])` mostra `0,3`. Outros resultados mantêm todos os dígitos: `raiz(2)` é `1,4142135623730951`. Com `define_opcao(:tipo_numero, :float)`, o valor binário exato é mantido: `seno(radianos(30))` mostra `0,49999999999999994`.

Toda função nativa cujo nome tem uma palavra acentuada aceita as duas grafias, como `maiuscula`/`maiúscula` e `laco_grafico`/`laço_gráfico`.
