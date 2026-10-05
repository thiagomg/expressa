# Expressa — folha de consulta

Arquivos `.lep`. Palavras em português. Índices começam em **1**.  
Detalhes: `especificacao-linguagem.md`. Nativas: `docs/nativas/*.lep`.

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
| par | `:nome -> "Ana"` |
| mapa | `mapa([:nome -> "Ana"])` |
| matriz | `matriz([[1, 2], [3, 4]])` só números; `importe "matriz"` |
| conjunto | `conjunto([:ana, 1])` valores únicos |

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
chave -> valor                 // par; p:chave  p:valor
e  ou  nao
lista contem 20
"Maria Silva" contem "Silva"
mapa contem "idade"
```

```text
mat::soma(1, 2)                // nome no módulo
pessoa:nome                    // chave de mapa (= pessoa["nome"]); mesma linha
xs.tamanho()                   // igual a tamanho(xs)  — o () é obrigatório
```

---

## Controle

```text
se nota >= 7 { "aprovado" } senao { "reprovado" }   // como valor, senao obrigatório
se n < 0 { escreva("negativo") }                    // comando: senao opcional

retorne verdadeiro                 // só numa função; sai na hora
retorne                            // devolve nada

repita 3 vezes { escreva("olá") }

para i de 1 ate 5 { escreva(i) }
para nome em ["Ana", "Bruno"] { escreva(nome) }
para ch em "olá" { escreva(ch) }    // texto: um caractere por vez
pare                               // sai do laço
continue                           // próxima volta (também continua)

i += 1                         // números: += e -=
nome += " Silva"               // texto: só +=

i = 1
enquanto i <= 3 {
    escreva(i)
    i = i + 1
}

valor = 10 / 0 se_falhar 0
importe "arquivo"
linhas = leia_arquivo("x.txt") se_falhar []
```

Sem `se_falhar`, um erro encerra o programa.

---

## Funções

Última expressão do bloco é o resultado. `retorne` sai mais cedo.

```text
soma = funcao(x, y) { x + y }
escreva(soma(10, 5))           // 15

avaliar("2 + 3 * 4")           // 14  (usa x, funções… do escopo atual)
escreva(avaliar(leia("> ")) se_falhar "conta inválida")
catalogo()                     // nativas, funcao do programa, módulos
catalogo(c)  catalogo("matriz")  // mapa, módulo ou função
ajuda("escreva")  ajuda("matriz::zeros")
                               // no REPL: ls   ls matriz   ls c

busca = funcao(xs, alvo) {
    para x em xs {
        se x == alvo { retorne verdadeiro }
    }
    falso
}
```

---

## Listas, textos, mapas, matrizes

```text
xs = [10, 20, 30, 40]
xs[1]                          // 10
xs[2..3]                       // [20, 30]
xs[2..]                        // até o fim
"b"[1..3]                      // "b" (não falha)
xs + [50]
xs += [50]                     // xs = xs + [50]
xs = xs.remova(2)               // sem o índice 2
xs = xs.remova(2, 4)            // faixa inclusiva
tamanho(xs)  primeiro(xs)  ultimo(xs)

t = "  Maria Silva  "
t[1]  t[1..5]
t[2] = "x"                     // um caractere; reatribui t
tamanho(t)  maiuscula(t)  minuscula(t)  limpe(t)  sem_acento(t)
substitua(t, "Maria", "Ana")
procurar("onde", "n")          // 2;  0 se não achar
procurar([10, 20, 30], 20)     // 2
separe("a,b,c", ",")           // ["a", "b", "c"]
junte(["a", "b"], " - ")
formate("{:<8} {:>5}", "Ana", 10)  // colunas: < esquerda  > direita  ^ centro

:nome                          // o texto "nome"
p = :nome -> "Ana"
p:chave  p:valor
pessoa = mapa([:nome -> "Ana", :idade -> 25])
pessoa:nome  pessoa[:nome]  pessoa["nome"]
pessoa:nome = "Bia"
pessoa["cidade"] = "Fortaleza"
pessoa = pessoa.remova("idade")

s = conjunto([:ana, :bia])
s contem :ana
s += :carlos
s = s.remova(:bia)

importe "matriz"
A = matriz([[1, 2, 3], [4, 5, 6]])
A[1, 2]                        // 2
A[1]                           // linha → lista
A + B    3 * A    A * B
transposta(A)  det(A)  identidade(3)
zeros(2, 5)  uns(3)  cheia(2, 3, 7)
nlinhas(A)  ncolunas(A)        // 2 e 3
tamanho(A)                     // também linhas
```

---

## Entrada e saída

```text
escreva("Média:", 7.5)         // stdout
escreva_erro("falhou")         // stderr (não entra em > arquivo)
durma(1)                       // espera 1 segundo  (aceita 0.5)
sair()                         // encerra, código 0
sair(1)                        // encerra com falha (não pega se_falhar)

nome = leia("Seu nome: ")      // uma linha, sem o Enter
idade = numero(leia("Idade: ")) se_falhar 0    // também número()
linhas = leia_linhas()         // resto da entrada → lista; vazio = []

busca = argumentos[1]          // depois do .lep; REPL → []

importe "tela"
cls()                          // limpa a tela  (também limpe_tela())
casa()                         // cursor no canto, sem apagar (animações)
eh_terminal()                  // teclado?  também é_terminal()
"HP".pinte(:verde)             // letra; :branco, :vermelho = letra e fundo
"    ".fundo(:azul)            // só o papel
"GO".negrito()
// cores: normal preto vermelho verde amarelo azul magenta ciano branco
```

```text
formato("pt")                  // 1.000,5   (padrão)
formato("en")                  // 1,000.5
numero("3,14")                 // segue o formato atual; também número()

importe "mat"
aleatorio(1, 6)                // inteiro inclusive; também aleatório()
semente(1)                     // mesma sequência de aleatorio
raiz(9)                        // 3
pi                             // valor (não pi())
seno(radianos(30))             // também sen, cosseno/cos, tangente/tan
graus(pi)                      // 180
log10(1000)                    // 3; log é o natural
potencia(2, 3)                 // 8
piso(3.7)  teto(3.1)  arredonde(1.5)
```

Literais no código sempre com ponto: `3.14`.

```text
importe "arquivo"
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

importe "matriz"               // nativo: construtor + zeros, transposta…
matriz = importe "matriz"      // alias: matriz::zeros, A.matriz::transposta()
importe "./matriz"             // arquivo local se o nativo colidir
```

Nativos: `matriz`, `arquivo`, `tela`, `mat`. Nome nu sem `/` nem `.lep` procura o nativo; se existir também `matriz.lep` no mesmo diretório, `importe "matriz"` é erro.

---

## Nativas (resumo)

Núcleo (sempre no escopo):

| | |
|---|---|
| `escreva` `escreva_erro` `sair` `durma` | imprimir / encerrar / esperar |
| `leia` `leia_linhas` | teclado / pipe |
| `argumentos` | lista (não é função) |
| `numero` `número` `formato` `avaliar` `catalogo` `ajuda` | número / executar texto / listar / ficha |
| `mapa` `conjunto` | construtores |
| `tamanho` `primeiro` `ultimo` | coleção |
| `maiuscula` `minuscula` `sem_acento` `remova` `substitua` `procurar` `separe` `junte` `limpe` `formate` | texto / lista / mapa / conjunto |

Módulos (`importe "…"`):

| | |
|---|---|
| `tela` | `cls` `casa` `eh_terminal` `pinte` `fundo` `negrito` |
| `mat` | `pi` `raiz` `aleatorio` `semente` `seno` `cosseno` `tangente` `arcoseno` `arcocosseno` `arcotangente` `arcotangente2` `radianos` `graus` `exp` `log` `log10` `potencia` `piso` `teto` `arredonde` |
| `matriz` | `matriz` `transposta` `det` `identidade` `zeros` `uns` `cheia` `nlinhas` `ncolunas` |
| `arquivo` | `leia_arquivo` `salve_arquivo` `adicione_arquivo` `leia_csv` `salve_csv` |

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
ajuda    ajuda funcoes    ajuda linguagem    ajuda escreva    ajuda matriz::zeros
cls                       // comando do REPL: limpa a tela
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
