# Tutorial avançado

Você já fez o [tutorial básico](basico.md) (`01`–`08` e o projeto). Agora aprenderá mapa, texto, `enquanto`, funções que retornam valor, erro, módulo, arquivo, tela.

Mesma regra: execute o `.lep`, mude uma linha e faça o desafio no fim. A [folha](../folha-de-consulta.md) e o [manual](../docs/manual.md) são para ver as funções e suas definições, não para ler em sequência.

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
escreva(s[1])          // E
escreva(s[1..4])       // Expr
escreva(s contem "press")
escreva(procurar(s, "s"))   // 6;  0 se não achar
```

`separe` gera uma lista baseado num texto; `junte` gera um texto baseado numa lista.

```text
expressa> "A B C".separe(" ")
["A", "B", "C"]
expressa> separe("abacate", "a")
["", "b", "c", "te"]
expressa> ["A", "B", "C"].junte("-")
"A-B-C"
```

`sem_acento("Ação")` vira `Acao` — a Forca usa isso para C e Ç.

Para mais exemplos: `../exemplos/texto/poema.lep`. Palíndromo no braço: `../exemplos/texto/palindromo.lep`.

**Desafio:** `separe` seu nome completo por espaço e escreva só o primeiro nome (`primeiro`).

---

## 11. Enquanto, pare, continue

Arquivo: `exemplos/11_enquanto.lep`

`para` tem a condição de saída do laço predeterminada. `enquanto` pergunta de novo a cada volta.

O laço mais simples conta até 5:

```text
i = 1
enquanto i <= 5 {
    escreva(i)
    i += 1
}
```

`+=` e `i = i + 1` são a mesma coisa. `continue` pula o resto desta volta e testa a condição de novo. `pare` sai do laço na hora.

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

A **última expressão** do bloco é o resultado. `retorne verdadeiro` sai no meio (útil num `para` que já achou o que queria).

Função **enxerga** nomes de fora e **não altera**. Para devolver um número novo, use o resultado: `x = soma(x, 1)`.

O arquivo também tem `tem(xs, alvo)`, que percorre a lista e usa `retorne` quando acha o valor.

Tabuada: `exemplos/07_funcao.lep`.
Super avançado — função que recebe função: `../exemplos/linguagem/filtra.lep`.

**Desafio:** implemente `maximo(a, b)`. Essa função deverá retornar o maior dos dois números.

---

## Tipo: `tipo(x)`

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

## 13. Ponto: `s.tamanho()`

Arquivo: `exemplos/13_ufcs.lep`

O primeiro argumento pode ir na frente do ponto. `tamanho(s)` e `s.tamanho()` são iguais. O `()` no fim é obrigatório.

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

## 14. Conjunto

Arquivo: `exemplos/14_conjunto.lep`

```text
visto = conjunto(["a", "b"])
visto += "a" // "a" é repetido. Tamanho continua 2
escreva(visto contem "a")
```

Cada valor entra somente uma vez. A Forca usa conjunto para verificar quais letras já foram usadas.

**Desafio:** tente adicionar três vezes o mesmo valor; o tamanho tem que ser 1.

---

## 15. Tratamento de erro: `se_falhar`

Arquivo: `exemplos/15_se_falhar.lep`

```text
escreva(10 / 0 se_falhar "não dá para dividir")
escreva([1, 2][9] se_falhar "índice longe")
escreva(numero("xyz") se_falhar 0)
```

Se você remover `se_falhar`, o programa encerra. Com `se_falhar`, o valor da direita é usado.

Mais casos (arquivo, índice): `../exemplos/linguagem/erros.lep`.

**Desafio:** `leia_arquivo("nao_existe.txt") se_falhar []` (precisa `importe "arquivo"`).

---

## 16. Módulos: `importe`

Execute `../exemplos/linguagem/usa_matematica.lep` **nessa pasta** (`exemplos/linguagem/`). O `importe "../lib/matematica"` só acha a biblioteca a partir dali.

Biblioteca importada: `../exemplos/lib/matematica.lep`.

```text
// Caso 1 — com apelido
mat = importe "../lib/matematica"
escreva(mat::media([7, 8, 9]))

// Caso 2 — nomes soltos no arquivo
importe "../lib/matematica"
escreva(soma(10, 32))
```

No caso 1, o arquivo vira `mat`, então os nomes levam `::`.  
No caso 2, as funções entram diretamente no arquivo.

Nativos (`"tela"`, `"mat"`, `"matriz"`, `"arquivo"`) não têm arquivo seu: `importe "mat"` basta, de qualquer pasta.

**Desafio:** no `usa_matematica.lep`, depois do caso 1 escreva `mat::minimo(notas)` e `mat::maximo(notas)`. Depois do caso 2, `minimo(notas)` e `maximo(notas)`. Essas funções já existem em `matematica.lep`.

---

## 17. Arquivo

`importe "arquivo"` lê e grava texto. O caminho é relativo à **pasta do `.lep`**, não à pasta de onde você chamou `expressa`.

```text
importe "arquivo"

salve_arquivo("saida/notas.txt", ["7", "8", "9"])
linhas = leia_arquivo("saida/notas.txt") se_falhar []
escreva(linhas)
```

Se o arquivo não existir, `leia_arquivo` é um erro. Com `se_falhar []` você continua com lista vazia.

No REPL (a pasta atual conta como pasta do programa):

```text
expressa> importe "arquivo"
expressa> salve_arquivo("oi.txt", ["olá"])
expressa> leia_arquivo("oi.txt")
["olá"]
```

Diário (grava e relê): `../exemplos/programas/diario.lep`. Agenda CSV: `../exemplos/programas/contatos.lep`.

No expressa-aula o programa só vê a pasta do projeto.

**Desafio:** execute o diário e abra `exemplos/programas/saida/diario.txt`.

---

## 18. Tela: cor e tamanho

Arquivo: `exemplos/16_tela.lep`

`importe "tela"`  
contém funções para pintar a tela e tamanho do terminal.

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

`quadro` + `bloco` + `escreva_em` montam a tela de um jogo (Termo, Forca). Desenhos grandes: `../exemplos/jogos/bicho_desenhos.lep` no terminal (o painel do expressa-aula mente 80×24).

**Desafio:** Escreva uma linha com o texto em verde com fundo branco.

---

## 19. Matemática (`mat`)

Arquivo: `exemplos/17_mat.lep`

```text
importe "mat"

escreva(raiz(9))
escreva(aleatorio(1, 6))
escreva("pi ≈ " + pi)
escreva("seno de 30°: " + seno(radianos(30)))
escreva(potencia(2, 10))
```

`pi` é valor, não função: escreva `pi`, não `pi()`. Seno e cosseno pedem **radianos**; `radianos(30)` converte graus.

```text
expressa> importe "mat"
expressa> pi
expressa> seno(radianos(90))
```

Contas de caderno: `../exemplos/escola/`.

**Desafio:** Mostre no terminal a área do círculo de raio 3 (`pi * r * r`).

---

## Plano cartesiano

Arquivo: `../exemplos/escola/seno_cosseno.lep`

`importe "cartesiano"` desenha *y = f(x)* como no caderno (y para cima). O **documento** é a janela; `mostre` atualiza a mesma.

```text
importe "mat"
cart = importe "cartesiano"

c = cart::documento()
g = cart::grafico()
g.cart::eixos(-2 * pi, 2 * pi, -1.5, 1.5)
g.cart::plotar(função(x) { seno(x) }, :azul)
c.cart::adicione(g)
c.cart::mostre()
```

Na Aula o gráfico aparece no painel; **Destacar** abre janela à parte. No terminal, `expressa` chama `expressa-grafico`.

**Desafio:** No mesmo documento, acrescente a parábola `x * x` em verde.

---

## 20. Argumentos da linha de comando

Arquivo: `exemplos/18_argumentos.lep`

O que vem depois do `.lep` entra na lista `argumentos`. O primeiro é `argumentos[1]`.

```text
expressa tutorial/exemplos/18_argumentos.lep Ana 16
```

Saída: `olá, Ana` e `idade: 16`. Sem argumentos, o programa pede o nome.

No expressa-aula, a caixa **argumentos** em cima é a mesma lista.

Exemplo avançado - filtrar linhas: `../exemplos/unix/grep.lep`.

**Desafio:** execute 18_argumentos.lep com dois nomes e escreva os dois.

---

## 21. Matriz

Estrutura bidimensional que só aceita números

`importe "matriz"`. `A[1, 2]` é linha 1, coluna 2.

```text
importe "matriz"

A = matriz([[1, 2, 3], [4, 5, 6]])
escreva(A[1, 2])        // 2
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

**Desafio:** `identidade(3)` e `escreva` a matriz.

---

## 22. Função que recebe função

Arquivo: `../exemplos/linguagem/filtra.lep`

Você passa o **teste** como argumento. `filtra` percorre a lista e fica com os itens para os quais o teste dá verdadeiro.

```text
pares = filtra([1, 2, 3, 4], função(n) { n % 2 == 0 })
escreva(pares)    // [2, 4]
```

O `%` é o resto da divisão: `n % 2 == 0` são os pares.

Closures (função que devolve função): `../exemplos/linguagem/closures.lep`.

**Desafio:** filtre as notas `>= 7` de `[5, 8, 6, 9]`.

---

## O que mais?

| O que | Onde |
|---|---|
| jogos | `../exemplos/jogos/` (Termo, Forca, Conway, adivinhe) |
| padaria, boletim, calculadora | `../exemplos/programas/` |
| referência de cada função | [manual](../docs/manual.md) · expressa-aula **F1** · `ajuda("escreva")` |
| cartão de uma página | [folha-de-consulta.md](../folha-de-consulta.md) |
| regra fina da linguagem | [especificacao-linguagem.md](../especificacao-linguagem.md) |
