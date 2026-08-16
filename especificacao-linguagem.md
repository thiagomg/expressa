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
Todo bloco (`inicio`/`fim`) é uma expressão e retorna o valor da última expressão.

---

## 1. Tipos Básicos

| Tipo     | Exemplos                          |
|----------|-----------------------------------|
| `numero` | `10`, `3.14`, `-8`, `0`           |
| `texto`  | `"olá"`, `"123"`                  |
| `bool`   | `verdadeiro`, `falso`             |

Tipos compostos: **lista** e **mapa**.

---

## 2. Variáveis e Escopo

- Declaração: apenas `nome = valor`
- Reatribuição permitida
- Escopo de bloco: variáveis criadas dentro de `inicio...fim` só existem dentro dele
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
```

---

## 6. Condicionais

Cada ramo tem seu próprio `inicio`/`fim`.

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

## 12. Arquivos

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

## 13. Tratamento de Erros

- Qualquer erro causa **crash** (com valores + call stack)
- Para tratar, usa-se `senao`

```text
linhas = leia_arquivo("arquivo.txt") senao []

valor = 10 / 0 senao 0

item = lista[99] senao "não existe"
```

---

## 14. Módulos / Importação

```text
mat = importe "matematica"     // com namespace
mat.soma(10, 5)

importe "matematica"           // sem namespace (traz tudo)
soma(10, 5)
```

---

## 15. Execução do Programa

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
