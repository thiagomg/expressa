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
| `numero` | `10`, `3.14`, `-8`, `0`           |
| `texto`  | `"olá"`, `"123"`                  |
| `bool`   | `verdadeiro`, `falso`             |

Tipos compostos: **lista**, **mapa** e **matriz**.

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
```

---

## 4. Operadores

**Aritméticos:** `+` `-` `*` `/` `%`  
**Comparação:** `==` `!=` `>` `<` `>=` `<=`  
**Lógicos:** `e` `ou` `nao`  
**Raiz quadrada (nativa):** `raiz(n)` — erro se `n < 0` (tratável com `se_falhar`)  
**Texto ↔ número:** `numero(t)` lê o padrão atual; `formato("pt")` / `formato("en")` escolhe pt-BR (`1.000,5`) ou en-US (`1,000.5`). Padrão pt-BR. Literais no código usam `.`.

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

Retangulares, só números. Índices começam em 1: `A[linha, coluna]`. `A[i]` devolve a linha como lista.

```text
A = matriz {
    [1, 2, 3],
    [4, 5, 6]
}

A[1, 2]          // 2
A + B            // mesma ordem
3 * A
A * B            // produto de matrizes
transposta(A)
det(A)           // 1×1, 2×2 ou 3×3
identidade(3)
tamanho(A)       // linhas
tamanho(A[1])    // colunas
```

---

## 6. Condicionais

Cada ramo tem seu próprio bloco (`inicio`/`fim` ou `{`/`}`).

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
```

---

## 8. Funções (primeira classe)

```text
soma = funcao(x, y)
inicio
    x + y
fim

escreva(soma(10, 5))     // 15
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
numeros contem 20              // verdadeiro
primeiro(numeros)              // 10
ultimo(numeros)                // 40
numeros[2..3]                  // [20, 30]
```

---

## 10. Mapas

```text
pessoa = mapa
inicio
    "nome" = "Ana"
    "idade" = 25
    "ativo" = verdadeiro
fim

pessoa["nome"]                 // "Ana"
pessoa contem "idade"          // verdadeiro
pessoa["cidade"] = "Fortaleza"
tamanho(pessoa)                // 4
```

---

## 11. Textos (strings)

```text
nome = "  Maria Silva  "

tamanho(nome)
maiuscula(nome)
minuscula(nome)
nome contem "Silva"
substitua(nome, "Maria", "Ana")
separe("a,b,c", ",")
junte(["a", "b"], " - ")
limpe(nome)
nome[1]
nome[1..5]
```

---

## 12. Entrada do teclado

`leia()` lê uma linha do teclado (sem a quebra de linha).  
`leia(prompt)` imprime o texto do prompt e em seguida espera a linha.

```text
nome = leia("Qual o seu nome? ")
escreva("Olá, " + nome)
```

Fim da entrada (Ctrl+D / Ctrl+Z) é um erro, tratável com `se_falhar`.

---

## 13. Arquivos

```text
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

## 14. Tratamento de Erros

- Qualquer erro causa **crash** (com valores + call stack)
- Para tratar, usa-se `se_falhar`

```text
linhas = leia_arquivo("arquivo.txt") se_falhar []

valor = 10 / 0 se_falhar 0

item = lista[99] se_falhar "não existe"
```

---

## 15. Módulos / Importação

```text
mat = importe "matematica"     // com namespace
mat.soma(10, 5)

importe "matematica"           // sem namespace (traz tudo)
soma(10, 5)
```

---

## 16. Execução do Programa

O código executa **de cima para baixo**, linha por linha.  
Não existe função `main` obrigatória.

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
