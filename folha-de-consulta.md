# Expressa — folha de consulta

Arquivos `.lep`. Palavras em português. Índices começam em **1**.  
Detalhes: `especificacao-linguagem.md`, `funcoes-nativas.md`.

```text
expressa                      REPL
expressa prog.lep [args…]
expressa debug prog.lep
cat dados.txt | expressa prog.lep Thiago
```

```text
#!/usr/bin/env expressa
chmod +x prog.lep && ./prog.lep
```

---

## Tipos

| Tipo | Exemplos |
|------|----------|
| `numero` | `10` `3.14` `-8` (no código, decimal com `.`) |
| `texto` | `"olá"` |
| `bool` | `verdadeiro` `falso` |
| lista | `[1, 2, 3]` |
| mapa | `mapa { "nome" -> "Ana" }` |
| matriz | `matriz { [1, 2], [3, 4] }` só números |

Não há literal `nada`. `escreva` e gravar arquivo devolvem `nada`.

---

## Variáveis, blocos, comentários

```text
x = 10                         // cria ou altera
inicio                         // ou { … }
    y = 20                     // some no fim do bloco
    escreva(x)
fim

resultado = { 10 + 5 }         // bloco é expressão → 15
```

Funções **leem** nomes de fora, mas **não alteram**.

```text
// uma linha
/* várias linhas */
#!/usr/bin/env expressa        // só na 1ª linha; # não é comentário
```

`inicio` fecha com `fim`; `{` fecha com `}`. Não misture.

---

## Operadores

```text
+  -  *  /  %                  // + também concatena se um lado é texto
==  !=  >  <  >=  <=           // sem a < b < c; use a < b e b < c
e  ou  nao
lista contem 20
"Maria Silva" contem "Silva"
mapa contem "idade"
```

```text
mat::soma(1, 2)                // nome no módulo
pessoa:nome                    // chave de mapa (= pessoa["nome"])
xs.tamanho()                   // igual a tamanho(xs)  — o () é obrigatório
```

---

## Controle

```text
se nota >= 7 { "aprovado" }
ou se nota >= 5 { "recuperação" }
senao { "reprovado" }

repita 3 vezes { escreva("olá") }

para i de 1 ate 5 { escreva(i) }
para nome em ["Ana", "Bruno"] { escreva(nome) }

i = 1
enquanto i <= 3 {
    escreva(i)
    i = i + 1
}

valor = 10 / 0 se_falhar 0
linhas = leia_arquivo("x.txt") se_falhar []
```

Não há `pare` / `continue`. Sem `se_falhar`, o programa para.

---

## Funções

Última expressão do bloco é o resultado. Sem `retorne`.

```text
soma = funcao(x, y) { x + y }
escreva(soma(10, 5))           // 15
```

---

## Listas, textos, mapas, matrizes

```text
xs = [10, 20, 30, 40]
xs[1]                          // 10
xs[2..3]                       // [20, 30]
xs + [50]
tamanho(xs)  primeiro(xs)  ultimo(xs)

t = "  Maria Silva  "
t[1]  t[1..5]
tamanho(t)  maiuscula(t)  minuscula(t)  limpe(t)
substitua(t, "Maria", "Ana")
separe("a,b,c", ",")           // ["a", "b", "c"]
junte(["a", "b"], " - ")

pessoa = mapa { "nome" -> "Ana", "idade" -> 25 }
pessoa:nome  pessoa["nome"]
pessoa:nome = "Bia"
pessoa["cidade"] = "Fortaleza"

A = matriz { [1, 2, 3], [4, 5, 6] }
A[1, 2]                        // 2
A[1]                           // linha → lista
A + B    3 * A    A * B
transposta(A)  det(A)  identidade(3)
tamanho(A)                     // linhas
```

---

## Entrada e saída

```text
escreva("Média:", 7.5)         // stdout
escreva_erro("falhou")         // stderr (não entra em > arquivo)
sair()                         // encerra, código 0
sair(1)                        // encerra com falha (não pega se_falhar)

nome = leia("Seu nome: ")      // uma linha, sem o Enter
idade = numero(leia("Idade: ")) se_falhar 0
linhas = leia_linhas()         // resto da entrada → lista; vazio = []

busca = argumentos[1]          // depois do .lep; REPL → []
eh_terminal()                  // teclado?  também é_terminal()
```

```text
formato("pt")                  // 1.000,5   (padrão)
formato("en")                  // 1,000.5
numero("3,14")                 // segue o formato atual
```

Literais no código sempre com ponto: `3.14`.

```text
linhas = leia_arquivo("dados.txt") se_falhar []
salve_arquivo("saida.txt", linhas)
adicione_arquivo("saida.txt", ["nova"])

dados = leia_csv("pessoas.csv")    // lista de listas de texto
salve_csv("saida.csv", dados)
```

Caminhos relativos: pasta do `.lep`, não a pasta de onde você chamou `expressa`.

---

## Módulos

```text
mat = importe "matematica"
mat::soma(10, 5)

importe "matematica"           // nomes no escopo atual
soma(10, 5)
```

---

## Nativas (resumo)

| | |
|---|---|
| `escreva` `escreva_erro` `sair` | imprimir / encerrar |
| `leia` `leia_linhas` `eh_terminal` | teclado / pipe |
| `argumentos` | lista (não é função) |
| `numero` `formato` `raiz` | número |
| `tamanho` `primeiro` `ultimo` | coleção |
| `maiuscula` `minuscula` `substitua` `separe` `junte` `limpe` | texto |
| `leia_arquivo` `salve_arquivo` `adicione_arquivo` | arquivo |
| `leia_csv` `salve_csv` | CSV |
| `transposta` `det` `identidade` | matriz |

Nativas não podem ser reatribuídas (`escreva = 1` é erro).

---

## Emacs

```elisp
(add-to-list 'load-path "/caminho/para/expressa/editors/emacs")
(require 'expressa-mode)
```

Abre `.lep` em `expressa-mode` (palavras-chave, nativas, comentários, strings).

---

## REPL e depurador

```text
expressa                  // ↑ ↓ histórico
ajuda    ajuda funcoes    ajuda linguagem    ajuda escreva
sair
```

```text
expressa debug prog.lep
continuar (c)   proximo (n)   entrar (s)   sair (o)
vars (v)   pilha (k)   ponto N   terminar (q)
```

---

## Exemplo

```text
#!/usr/bin/env expressa
busca = argumentos[1] se_falhar ""
para linha em leia_linhas() {
    se linha contem busca { escreva(linha) }
}
```

```text
cat nomes.txt | expressa grep.lep Thiago
```
