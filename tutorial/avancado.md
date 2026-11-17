# Tutorial avançado

| Material | O que você encontra |
|---|---|
| [Manual](../docs/manual.md) | referência da linguagem e das funções |
| [Tutorial básico](basico.md) | primeiros passos, capítulos 1 a 8 e projetos |
| **Tutorial avançado** | coleções, funções, módulos, arquivos e gráficos |

Este tutorial continua os capítulos do [tutorial básico](basico.md). Antes de começar, conclua os capítulos 1 a 8 e experimente um dos projetos. Aqui você vai trabalhar com mapas, textos, laços, funções com retorno, tratamento de erros, módulos, arquivos e gráficos.

Execute cada programa de exemplo, mude uma linha e faça o desafio no fim do capítulo. Consulte a [folha de consulta](../folha-de-consulta.md) e o [manual](../docs/manual.md) quando precisar lembrar uma operação ou entender uma função.

Os caminhos dos arquivos partem da pasta `tutorial/`, salvo quando o comando mostra o caminho completo a partir da raiz do repositório. Os números nos nomes dos arquivos identificam os exemplos e podem ser diferentes dos números dos capítulos.

## 9. Mapa

Arquivo: `exemplos/09_mapa.lep`

Um mapa liga uma **chave** a um **valor**. `:nome` é a chave (o texto `"nome"`). `pessoa:nome` e `pessoa["nome"]` são iguais.

```text
pessoa = mapa([
    :nome -> "Ana",
    :idade -> 16,
])
escreva(pessoa:nome)
escreva(pessoa:idade)

pessoa:cidade = "Fortaleza"
escreva(pessoa:cidade)

se pessoa contem "idade" {
    escreva("tem idade")
}
```

No REPL:

```text
expressa> pessoa = mapa([:nome -> "Ana", :idade -> 16])
expressa> pessoa:nome
"Ana"
expressa> pessoa["nome"]
"Ana"
expressa> pessoa contem "idade"
verdadeiro
expressa> pessoa contem "serie"
falso
```

Receita de bolo com mapa de quantidades: `../exemplos/programas/receita.lep`.

**Desafio:** acrescente `:serie -> 2` e escreva a série.

---

## 10. Interagindo com texto

Arquivo: `exemplos/10_texto.lep`

```text
s = "Expressa"
escreva(s[0])          // E
escreva(s[0..4])       // Expr
escreva(s[4..])        // essa
escreva(s contem "press")
escreva(procure(s, "s"))   // 5;  -1 se não achar
```

`separe` divide um texto em uma lista de partes, usando o separador indicado. `junte` faz o caminho inverso: une os itens de uma lista em um texto, colocando o separador entre eles.

```text
expressa> "A B C".separe(" ")
["A", "B", "C"]
expressa> separe("abacate", "a")
["", "b", "c", "te"]
expressa> ["A", "B", "C"].junte("-")
"A-B-C"
```

`sem_acento("Ação")` retorna `"Acao"`. A Forca usa essa função para tratar letras como C e Ç da mesma forma.

Para mais exemplos: `../exemplos/texto/poema.lep`. Verificação de palíndromos passo a passo: `../exemplos/texto/palindromo.lep`.

**Desafio:** use `separe` para dividir seu nome completo nos espaços e `primeiro` para mostrar apenas o primeiro nome.

---

## 11. Enquanto, pare, continue

Arquivo: `exemplos/11_enquanto.lep`

`para` percorre os itens de uma coleção ou faixa. Já `enquanto` testa uma condição antes de cada repetição e continua enquanto ela for verdadeira.

O laço mais simples conta até 5:

```text
i = 1
enquanto i <= 5 {
    escreva(i)
    i += 1
}
```

`i += 1` equivale a `i = i + 1`. `continue` pula o resto desta volta e testa a condição de novo. `pare` sai do laço na hora.

O arquivo `11_enquanto.lep` junta os dois: pula o 3 e para no 4. A tela mostra `1`, `2` e `4`.

```text
i = 1
enquanto i <= 5 {
    se i == 3 {
        i += 1
        continue
    }
    escreva(i)
    se i == 4 {
        pare
    }
    i += 1
}
```

Jogo que usa `enquanto` + `leia`: `../exemplos/jogos/adivinhe_o_numero.lep`.

**Desafio:** conte de 10 até 1.

---

## 12. Funções que retornam valor

Arquivo: `exemplos/12_retorno.lep`

```text
soma = função(a, b) {
    a + b
}
escreva(soma(10, 5))    // 15
```

O valor da **última expressão** do bloco é o resultado da função. Use `retorne` para encerrar a função antes de chegar ao fim, por exemplo, quando uma busca já encontrou o valor desejado.

Uma função pode ler variáveis definidas fora dela, mas não pode reatribuir essas variáveis. Para atualizar um número, retorne o novo valor e faça a atribuição fora da função: `x = soma(x, 1)`. Essa restrição é sobre a variável; elementos de listas, mapas e matrizes compartilhados podem ser alterados dentro de uma função.

O arquivo também tem `tem(xs, alvo)`, que percorre a lista e usa `retorne` quando acha o valor.

Tabuada: `exemplos/07_funcao.lep`.
Você também pode passar uma função como argumento de outra, como neste exemplo: `../exemplos/linguagem/filtra.lep`.

**Desafio:** implemente `maximo(a, b)`. Essa função deverá retornar o maior dos dois números.

---

## 13. Consultando o tipo de um valor

Arquivo: `exemplos/19_tipo.lep`

`tipo(valor)` devolve o nome do tipo como texto (`"numero"`, `"texto"`, `"lista"`, `"funcao"`, …). Assim uma função trata mais de um tipo:

```text
mostra = função(x) {
    se tipo(x) == "numero" {
        escreva(x * 2)
    } ou se tipo(x) == "texto" {
        escreva(x.maiuscula())
    } senão {
        escreva(x)
    }
}

mostra(10)
mostra("oi")
```

Os nomes são os mesmos das mensagens de erro (sem acento: `funcao`, não `função`). `tipos()` lista todos. No REPL: `tipo(10)`, `10.tipo()`, `tipos()`, `ajuda tipo`.

**Desafio:** se `x` for lista, escreva o `tamanho(x)`.

---

## 14. Chamando funções com ponto: `s.tamanho()`

Arquivo: `exemplos/13_ufcs.lep`

Você pode escrever o primeiro argumento antes do nome da função, separando os dois por um ponto. `tamanho(s)` e `s.tamanho()` são iguais. O `()` no fim é obrigatório.

```text
s = "olá"
escreva(tamanho(s))
escreva(s.tamanho())
escreva(s.maiuscula())
escreva([10, 20].primeiro())
```

Dá para encadear: cada ponto usa o resultado do anterior.

```text
expressa> "oi".maiuscula().tamanho()
2
```

**Desafio:** chame `primeiro([1, 2, 3])` nas duas formas (`primeiro(lista)` e `lista.primeiro()`). `primeiro` já existe; não precisa criar.

---

## 15. Conjuntos

Arquivo: `exemplos/14_conjunto.lep`

```text
visto = conjunto(["a", "b"])
visto += "a" // "a" é repetido. Tamanho continua 2
escreva(visto contem "a")
```

Cada valor entra somente uma vez. A Forca usa conjunto para verificar quais letras já foram usadas.

**Desafio:** crie um conjunto vazio e adicione o mesmo valor três vezes. Confira com `tamanho` que o conjunto contém apenas um item.

---

## 16. Tratando erros com `se_falhar`

Arquivo: `exemplos/15_se_falhar.lep`

```text
escreva(10 / 0 se_falhar { "não dá para dividir" })
escreva([1, 2][9] se_falhar { "índice fora dos limites" })
escreva(numero("xyz") se_falhar { 0 })
escreva(falhe("negativo") se_falhar { "erro tratado" })
```

Nesses exemplos, cada expressão gera um erro. Sem `se_falhar`, o erro encerra o programa. Com `se_falhar`, o bloco de tratamento é executado e seu resultado substitui o da expressão que falhou.

Para consultar a mensagem do erro, coloque um nome entre `se_falhar` e o bloco. Essa variável recebe a mensagem como texto e só existe dentro do bloco:

```text
valor = numero("abc") se_falhar erro {
    escreva("Falha:", erro)
    falso
}
```

O bloco é obrigatório, inclusive sem captura: `numero("abc") se_falhar { falso }`.
Se o próprio bloco de tratamento gerar outro erro, esse novo erro precisa ser tratado em outro `se_falhar`; caso contrário, ele encerra o programa.

`falhe("mensagem")` gera um erro com a mensagem informada. `garanta(n > 0)` gera um erro se a condição for falsa. Você pode tratar ambos com `se_falhar`.

Mais casos (arquivo, índice): `../exemplos/linguagem/erros.lep`.

**Desafio:** importe o módulo `arquivo` e tente ler um arquivo inexistente. Use `se_falhar { [] }` para obter uma lista vazia quando a leitura falhar.

---

## 17. Importando módulos

O programa `../exemplos/linguagem/usa_matematica.lep` importa a biblioteca `../exemplos/lib/matematica.lep`. O caminho da importação é relativo à pasta do arquivo que faz o `importe`.

Na raiz do repositório, execute:

```text
expressa exemplos/linguagem/usa_matematica.lep
```

```text
// Caso 1 — com apelido
mat = importe "../lib/matematica"
escreva(mat::media([7, 8, 9]))

// Caso 2 — funções disponíveis sem prefixo
importe "../lib/matematica"
escreva(soma(10, 32))
```

No caso 1, `mat` é o apelido do módulo. Para acessar uma função dele, use esse apelido seguido de `::`, como em `mat::media(...)`.

No caso 2, as funções ficam disponíveis diretamente no escopo da importação, sem prefixo. Se algum nome já estiver em uso por outra importação ou variável, ocorre um erro. Nesse caso, importe o módulo com um apelido.

Módulos nativos, como `tela`, `mat`, `matriz`, `arquivo` e `crono`, fazem parte da Expressa. Você pode usar `importe "mat"` sem criar um arquivo para esse módulo. Se existir um arquivo local com o mesmo nome, a importação é ambígua: renomeie o arquivo ou use `importe "./mat"` para escolher o arquivo local.

**Desafio:** no `usa_matematica.lep`, depois do caso 1 escreva `mat::minimo(notas)` e `mat::maximo(notas)`. Depois do caso 2, `minimo(notas)` e `maximo(notas)`. Essas funções já existem em `matematica.lep`.

---

## 18. Lendo e gravando arquivos

O módulo `arquivo` oferece funções para ler e gravar textos. O caminho é relativo à **pasta do `.lep`**, não à pasta de onde você chamou `expressa`.

```text
importe "arquivo"

salve_arquivo("saida/notas.txt", ["7", "8", "9"])
linhas = leia_arquivo("saida/notas.txt") se_falhar { [] }
escreva(linhas)
```

Se o arquivo não existir, `leia_arquivo` gera um erro. Com `se_falhar { [] }`, o programa continua e usa uma lista vazia como resultado.

No REPL (a pasta atual conta como pasta do programa):

```text
expressa> importe "arquivo"
expressa> salve_arquivo("oi.txt", ["olá"])
expressa> leia_arquivo("oi.txt")
["olá"]
```

Diário (grava e relê): `../exemplos/programas/diario.lep`. Agenda CSV: `../exemplos/programas/contatos.lep`.

No expressa-aula, os caminhos relativos partem da pasta do projeto. Use `~` para indicar sua pasta pessoal, como em `~/pasta/arquivo.txt`.

**Desafio:** execute o diário e abra `exemplos/programas/saida/diario.txt`.

---

## 19. Cores e tamanho do terminal

Arquivo: `exemplos/16_tela.lep`

O módulo `tela` permite colorir textos, posicionar o cursor e consultar o tamanho do terminal.

```text
importe "tela"

escreva("ok".pinte(:verde))
escreva("alerta".pinte(:preto, :amarelo))
escreva("fim".negrito().pinte(:vermelho))
escreva("colunas: " + colunas() + "  linhas: " + linhas())
```

Cores: `preto vermelho verde amarelo azul magenta ciano branco`.  
`pinte(texto, frente)` ou `pinte(texto, frente, fundo)`.

No REPL:

```text
expressa> importe "tela"
expressa> "HP".pinte(:verde)
expressa> colunas()
```

As funções `quadro`, `bloco` e `escreva_em` ajudam a organizar a tela de jogos como Termo e Forca. Experimente `../exemplos/jogos/bicho_desenhos.lep` no terminal para ver desenhos maiores. No painel do expressa-aula, `colunas()` e `linhas()` informam valores fixos de 80 colunas e 24 linhas.

**Desafio:** escreva uma linha com texto verde sobre fundo branco.

---

## 20. Matemática com o módulo `mat`

Arquivo: `exemplos/17_mat.lep`

```text
importe "mat"

escreva(raiz(9))
escreva(aleatorio(1, 6))
escreva("pi ≈ " + pi)
escreva("seno de 30°: " + seno(radianos(30)))
escreva(potencia(2, 10))
```

`pi` é valor, não função: escreva `pi`, não `pi()`. Seno e cosseno recebem ângulos em **radianos**; `radianos(30)` converte graus.

```text
expressa> importe "mat"
expressa> pi
expressa> seno(radianos(90))
```

Contas de caderno: `../exemplos/escola/`.

**Desafio:** mostre no terminal a área do círculo de raio 3 (`pi * r * r`).

---

## 21. Gráficos no plano cartesiano

Arquivo: `../exemplos/escola/seno_cosseno.lep`

O módulo `cartesiano` desenha gráficos de funções *y = f(x)*, com o eixo y crescendo para cima. Um **documento** reúne os gráficos exibidos em uma janela. Chamadas de `mostre` para o mesmo documento atualizam essa janela.

```text
importe "mat"
cart = importe "cartesiano"

c = cart::documento()
g = cart::grafico()
g.cart::eixos(-2 * pi, 2 * pi, -1.5, 1.5)
g.cart::plote(função(x) { seno(x) }, :azul)
c.cart::adicione(g)
c.cart::mostre()
```

No expressa-aula, o gráfico aparece no painel **Gráficos**. Use **Destacar** para abrir uma janela separada. No terminal, execute o exemplo com `expressa-grafico` ou `expressa --grafico`.

**Desafio:** antes de adicionar o gráfico ao documento, acrescente a curva `função(x) { x * x }` em verde usando `plote`.

---

## 22. Argumentos da linha de comando

Arquivo: `exemplos/18_argumentos.lep`

O que vem depois do `.lep` entra na lista `argumentos`. O primeiro é `argumentos[0]`.

```text
expressa tutorial/exemplos/18_argumentos.lep Ana 16
```

A saída mostra `olá, Ana` e `idade: 16`. Sem argumentos, o programa mostra uma orientação para você executá-lo passando um nome.

No expressa-aula, preencha o campo **Argumentos** antes de executar. Os valores informados ficam disponíveis na mesma lista `argumentos`.

Outro exemplo que usa argumentos para filtrar linhas: `../exemplos/unix/grep.lep`.

**Desafio:** adapte `18_argumentos.lep` para receber dois nomes e mostrar uma saudação para cada um. Execute o programa passando os dois nomes como argumentos.

---

## 23. Matrizes

Uma matriz organiza números em linhas e colunas. Para trabalhar com matrizes, use `importe "matriz"`. O acesso `A[0, 1]` indica a linha 0 e a coluna 1.

```text
importe "matriz"

A = matriz([[1, 2, 3], [4, 5, 6]])
escreva(A[0, 1])        // 2
escreva(A.xy(1, 0))     // 2: x é coluna, y é linha
A.xy(1, 0, 9)    // altera A[0, 1]
escreva(nlinhas(A))     // 2
escreva(ncolunas(A))    // 3
escreva(identidade(3))
```

No REPL:

```text
expressa> importe "matriz"
expressa> identidade(2)
```

Tabela de frutas: `../exemplos/programas/frutas.lep`. Geometria escolar: `../exemplos/escola/geometria.lep`.

**Desafio:** crie uma matriz com `identidade(3)` e mostre o resultado com `escreva`.

---

## 24. Funções que recebem funções

Arquivo: `../exemplos/linguagem/filtra.lep`

Uma função também pode receber outra função como argumento. Neste exemplo, `filtra` recebe uma lista e uma função de **teste**. Ela percorre a lista e retorna apenas os itens para os quais o teste retorna `verdadeiro`.

`filtra` é definida no programa de exemplo; ela não é uma função nativa. Veja a definição completa:

```text
filtra = função(lista, teste) {
    resultado = []
    para item em lista {
        se teste(item) {
            resultado += [item]
        }
    }
    resultado
}

pares = filtra([1, 2, 3, 4], função(n) { n % 2 == 0 })
escreva(pares)    // [2, 4]
```

O operador `%` calcula o resto da divisão. O teste `n % 2 == 0` retorna `verdadeiro` quando `n` é par.

Para ver uma função que retorna outra função e preserva valores do contexto em que foi criada (uma *closure*), consulte: `../exemplos/linguagem/closures.lep`.

**Desafio:** filtre as notas `>= 7` de `[5, 8, 6, 9]`.

---

## O que mais?

| O que | Onde |
|---|---|
| jogos | `../exemplos/jogos/` (Termo, Forca, Conway, adivinhe) |
| padaria, boletim, calculadora | `../exemplos/programas/` |
| referência de cada função | [manual](../docs/manual.md) · expressa-aula **F1** · `ajuda("escreva")` |
| resumo da sintaxe e das funções | [folha-de-consulta.md](../folha-de-consulta.md) |
| regras detalhadas da linguagem | [especificacao-linguagem.md](../especificacao-linguagem.md) |
