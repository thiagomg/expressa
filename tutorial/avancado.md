# Tutorial avançado

Você já fez o [tutorial básico](basico.md) (`01`–`08`). Daqui para frente: mapa, texto, `enquanto`, funções que devolvem valor, erro, módulo, arquivo, tela.

Mesma regra: execute o `.lep`, mude uma linha, desafio no fim. A [folha](../folha-de-consulta.md) e o [manual](../docs/manual.md) são para consultar o nome de uma função, não para ler em sequência.

## 9. Mapa

Arquivo: `exemplos/09_mapa.lep`

```text
pessoa = mapa([
    :nome -> "Ana",
    :idade -> 16,
])
escreva(pessoa:nome)
pessoa:cidade = "Fortaleza"
```

Um mapa liga **chave** a **valor**. `:nome` é a chave (o texto `"nome"`). `pessoa:nome` e `pessoa["nome"]` leem a mesma coisa.

Receita de bolo com mapa de quantidades: `../exemplos/programas/receita.lep`.

**Desafio:** acrescente `:serie -> 2` e escreva a série.

---

## 10. Texto de verdade

Arquivo: `exemplos/10_texto.lep`

```text
s = "Expressa"
escreva(s[1])          // E
escreva(s[1..4])       // Expr
escreva(s contem "press")
escreva(procurar(s, "s"))   // 6;  0 se não achar
```

`separe` parte; `junte` cola. `sem_acento("Ação")` vira `Acao` — a Forca usa isso para C e Ç.

Oficina maior: `../exemplos/texto/poema.lep`. Palíndromo no braço: `../exemplos/texto/palindromo.lep`.

**Desafio:** `separe` seu nome completo por espaço e escreva só o primeiro nome (`primeiro`).

---

## 11. Enquanto, pare, continue

Arquivo: `exemplos/11_enquanto.lep`

`para` sabe o fim. `enquanto` pergunta de novo a cada volta.

```text
i = 1
enquanto i <= 5 {
    escreva(i)
    i += 1
}
```

`+=` é `i = i + 1`. `pare` sai do laço; `continue` pula o resto desta volta.

Jogo que usa `enquanto` + `leia`: `../exemplos/jogos/adivinhe_o_numero.lep`.

**Desafio:** conte de 10 até 1.

---

## 12. Função que devolve valor

Arquivo: `exemplos/12_retorno.lep`

```text
soma = função(a, b) {
    a + b
}
escreva(soma(10, 5))    // 15
```

A **última expressão** do bloco é o resultado. `retorne verdadeiro` sai no meio (útil num `para` que já achou o que queria).

Função **lê** nomes de fora e **não altera**. Para devolver um número novo, use o resultado: `x = soma(x, 1)`.

Tabuada (só efeito na tela): `exemplos/07_funcao.lep`. Alta ordem (função que recebe função): `../exemplos/linguagem/filtra.lep`.

**Desafio:** `maximo(a, b)` que devolve o maior dos dois.

---

## 13. Ponto: `s.tamanho()`

Arquivo: `exemplos/13_ufcs.lep`

```text
tamanho(s)
s.tamanho()     // igual; o () é obrigatório
s.maiuscula()
```

O primeiro argumento pode ir na frente do ponto. Encadear: `"oi".maiuscula().tamanho()`.

**Desafio:** `primeiro([1, 2, 3])` nas duas formas.

---

## 14. Conjunto

Arquivo: `exemplos/14_conjunto.lep`

```text
visto = conjunto(["a", "b"])
visto += "a"              // já estava: tamanho continua 2
escreva(visto contem "a")
```

Cada valor entra uma vez. A Forca usa conjunto para letras já acertadas.

**Desafio:** três tentativas repetidas; o tamanho tem que ser 1.

---

## 15. Quando dá erro: `se_falhar`

Arquivo: `exemplos/15_se_falhar.lep`

```text
escreva(10 / 0 se_falhar "não dá para dividir")
escreva(numero("xyz") se_falhar 0)
```

Sem `se_falhar`, o programa encerra. Com, usa o valor da direita.

Mais casos (arquivo, índice): `../exemplos/linguagem/erros.lep`.

**Desafio:** `leia_arquivo("nao_existe.txt") se_falhar []` (precisa `importe "arquivo"`).

---

## 16. Módulos: `importe`

Arquivo pronto: `../exemplos/linguagem/usa_matematica.lep`

```text
mat = importe "../lib/matematica"
escreva(mat::media([7, 8, 9]))

importe "../lib/matematica"
escreva(soma(10, 32))
```

Com alias, o nome leva `::`. Sem, as funções entram soltas no arquivo.

Nativos (não são arquivo seu): `"tela"`, `"mat"`, `"matriz"`, `"arquivo"`.

Biblioteca de aluno: `../exemplos/lib/matematica.lep`.

**Desafio:** no `usa_matematica.lep`, escreva também o mínimo da lista (já existe `minimo`).

---

## 17. Arquivo

`importe "arquivo"`. Caminho relativo: pasta do `.lep`.

```text
salve_arquivo("saida/notas.txt", ["7", "8", "9"])
linhas = leia_arquivo("saida/notas.txt") se_falhar []
```

Diário (grava e relê): `../exemplos/programas/diario.lep`. Agenda CSV: `../exemplos/programas/contatos.lep`.

Na Aula o programa só vê a pasta do projeto.

**Desafio:** execute o diário, abra `exemplos/programas/saida/diario.txt`.

---

## 18. Tela: cor e tamanho

Arquivo: `exemplos/16_tela.lep`

```text
importe "tela"
escreva("ok".pinte(:verde))
escreva("colunas: " + colunas())
```

Cores: `preto vermelho verde amarelo azul magenta ciano branco`. `pinte(texto, frente)` ou `pinte(texto, frente, fundo)`.

`quadro` + `bloco` + `escreva_em`: um quadro de jogo (Termo, Forca). Desenhos grandes: `../exemplos/jogos/bicho_desenhos.lep` no terminal (não na Aula 80×24).

**Desafio:** uma linha vermelha e uma verde.

---

## 19. Matemática (`mat`)

Arquivo: `exemplos/17_mat.lep`

```text
importe "mat"
escreva(raiz(9))
escreva(aleatorio(1, 6))
escreva(seno(radianos(30)))
```

`pi` é valor, não função: `pi`, não `pi()`. Contas de caderno: `../exemplos/escola/`.

**Desafio:** área do círculo de raio 3 (`pi * r * r`).

---

## 20. Argumentos da linha de comando

Arquivo: `exemplos/18_argumentos.lep`

```text
expressa tutorial/exemplos/18_argumentos.lep Ana 16
```

`argumentos[1]` é `Ana`. Na Aula, a caixa **argumentos**.

Filtro de linhas (tipo `grep`): `../exemplos/unix/grep.lep`.

**Desafio:** execute com dois nomes e escreva os dois.

---

## 21. Matriz

Só números. `importe "matriz"`. `A[1, 2]` é linha 1, coluna 2.

Tabela de frutas (imprime bonito): `../exemplos/programas/frutas.lep`. Geometria escolar: `../exemplos/escola/geometria.lep`.

**Desafio:** `identidade(3)` e `escreva` a matriz.

---

## 22. Função que recebe função

Arquivo: `../exemplos/linguagem/filtra.lep`

```text
pares = filtra([1, 2, 3, 4], função(n) { n % 2 == 0 })
```

Você passa o **teste** como argumento. Closures (função que devolve função): `../exemplos/linguagem/closures.lep`.

**Desafio:** filtre as notas `>= 7` de `[5, 8, 6, 9]`.

---

## E agora

| | |
|---|---|
| jogos | `../exemplos/jogos/` (Termo, Forca, Conway, adivinhe) |
| padaria, boletim, calculadora | `../exemplos/programas/` |
| referência de cada função | [manual](../docs/manual.md) · Aula **F1** · `ajuda("escreva")` |
| cartão de uma página | [folha-de-consulta.md](../folha-de-consulta.md) |
| regra fina da linguagem | [especificacao-linguagem.md](../especificacao-linguagem.md) |
