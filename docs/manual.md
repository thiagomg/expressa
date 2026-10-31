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
| processo | `linhas_de("ls")` | comando rodando; `importe "processo"` |
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

Nativos: `tela`, `mat`, `matriz`, `arquivo`, `cartesiano`, `tela_grafica`, `imagem`. Nome sem `/` nem `.lep` procura o nativo primeiro. `importe "./matriz"` força arquivo local.

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

### `tipo(valor)`

Nome do tipo, como texto: `numero`, `texto`, `bool`, `lista`, `par`, `mapa`, `conjunto`, `matriz`, `funcao`, `modulo`, `nada`. Iguais às mensagens de erro (sem acento). Nativa e função do aluno: `"funcao"`.

```text
se tipo(x) == "numero" { x * 2 } senão { x }
x.tipo()
tipos()    // lista dos nomes
```

### `tipos()`

Lista de textos: os nomes que `tipo()` pode devolver, nesta ordem.

```text
tipos()
tipos() contem "mapa"
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

Um mapa pode definir `:_como_texto` (função de um argumento que devolve texto). `formate`, `escreva` e o `+` com texto chamam essa função. Sem o campo, o mapa continua `mapa{…}`. A função é do aluno (`como_texto = função(fr) { … }`; `fr:_como_texto = como_texto`); não é nativa.

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

### `ordene(lista)` / `ordene(lista, chave)`

Cópia em ordem crescente. Todos números ou todos textos. Com chave: `ordene(pessoas, funcao(p) { p:idade })`.

### `comeca_com(texto, prefixo)` / `começa_com` · `termina_com(texto, sufixo)`

Verdadeiro ou falso.

### `ambiente(nome)`

Variável de ambiente. Não existe → erro: `ambiente("EDITOR") se_falhar "vim"`.

### `agora()` · `formate_data(data, modelo)`

`agora()` é um mapa `:ano` `:mes` `:dia` `:hora` `:minuto` `:segundo` `:dia_da_semana` (1 = domingo) `:epoch`. `formate_data(agora(), "%Y-%m-%d %H:%M:%S")`.

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

## `importe "cartesiano"`

Gráfico de função no plano xy (caderno: y para cima). O **documento** é a janela; `mostre` atualiza e foca a mesma.

```text
importe "mat"
cart = importe "cartesiano"

c = cart::documento()
g = cart::grafico()
g.cart::eixos(-2 * pi, 2 * pi, -1.5, 1.5)
g.cart::plotar(função(x) { seno(x) }, :azul)
g.cart::plotar(função(x) { cosseno(x) }, :vermelho)
c.cart::adicione(g)
c.cart::mostre()
```

`adicione` copia o gráfico (congelado). O próximo `mostre` do mesmo `c` não abre outra janela. Dois `documento()` são duas janelas. Cores: as da `tela`. `tipo(g)` é `"mapa"`. Na Aula aparece o painel **Gráficos** (botão Destacar). No terminal, `expressa-grafico` (ou `expressa --grafico`).

## `importe "tela_grafica"`

Tela de pixels (MSX SCREEN 2): um mapa por janela, comandos `linha` / `circulo` / `ponto` / `retangulo` / `imagem`, depois `mostre` ou `salve`. Cores da `tela` e `transparente`. `fundo(g, cor)` é o papel. `como_imagem(g)` vira um valor `imagem` (furos com alpha 0).

```
tela_grafica = importe "tela_grafica"
g = tela_grafica::nova(320, 200)
g.tela_grafica::cor(:azul)
g.tela_grafica::linha(10, 10, 310, 190)
g.tela_grafica::texto(10, 10, "casa")
g.tela_grafica::mostre()
g.tela_grafica::salve("casa.png")
```

`(0, 0)` é o canto superior esquerdo; y cresce para baixo. Cada `nova()` é uma janela; o `mostre` da mesma instância só atualiza. `texto(g, x, y, t)` escreve na caneta a partir de `(x, y)` (células de 8 pixels; `texto(..., 16)` dobra). `evento(g)` espera um clique ou tecla nessa janela; `evento(g, 0.05)` espera até 50 ms. Na Aula a janela é irmã (não entra no painel Gráficos). `pixel(g, x, y)` devolve a cor daquele ponto. Detalhe do protocolo com o visor: `docs/tela_grafica.md`.

## `importe "imagem"`

PNG em memória: `carregue`, `salve`, `rotacione`, `escala`, `largura`, `altura`. `tipo(img)` é `"imagem"`. `tela_grafica::imagem` aceita o valor, um caminho (lido uma vez) ou outra tela gráfica.

```
img = importe "imagem"
figura = img::carregue("tartaruga.png")
g.tela_grafica::imagem(figura, 160, 100, 90)
figura.img::salve("copia.png")
```

### `documento()`

Mapa-folha. Ainda não abre janela.

### `grafico()`

Plano vazio. Depois `eixos` e `plotar`.

### `eixos(grafico, xmin, xmax, ymin, ymax)`

`xmin < xmax`, `ymin < ymax`. Sem isso, `plotar` é erro.

### `plotar(grafico, f, cor)`

`f` recebe x e devolve y. ~400 amostras. y infinito vira um furo na linha.

### `adicione(documento, grafico)`

Pelo menos uma curva. Devolve o documento.

### `mostre(documento)`

Abre ou atualiza. Não espera fechar. Folha vazia é erro (`se_falhar`).

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

### `existe(caminho)` · `eh_pasta(caminho)` / `é_pasta`

Verdadeiro ou falso.

### `liste(pasta)` / `liste(pasta, padrao)`

Nomes dentro da pasta, em ordem, com os ocultos. Padrão: `*` qualquer trecho, `?` um caractere (`liste("fotos", "*.png")`).

### `crie_pasta(caminho)` · `apague_arquivo(caminho)` · `apague_pasta(caminho)`

`crie_pasta` cria as que faltam. `apague_pasta` apaga a pasta e tudo dentro (recusa `/` e a pasta pessoal).

### `mova(origem, destino)` · `copie(origem, destino)`

Se o destino é uma pasta, vai para dentro dela. `copie` só copia arquivos.

### `junte_caminho(parte, …)` · `nome_do_arquivo` · `pasta_de` · `extensao` · `pasta_atual()`

Só montam ou leem o texto do caminho. `pasta_atual()` é a pasta do terminal; os caminhos relativos das outras funções partem da pasta do `.lep`:

```text
entrada = junte_caminho(pasta_atual(), argumentos[1])
```

### `leia_csv(caminho)` / `salve_csv(caminho, dados)`

CSV simples (vírgula, sem aspas). Lista de listas de **texto**. `salve_csv` substitui o arquivo.

```text
dados = leia_csv("pessoas.csv")
salve_csv("saida.csv", [["Ana", 25]])
```

---

## `importe "processo"`

Roda outros programas. Só na linha de comando (na Aula é erro). Opções também `:exija -> verdadeiro` (código diferente de 0 vira erro, como `set -e`) e `:terminal -> verdadeiro` (o comando usa o teclado e a tela: `vim`, `ssh`, cores; só em `execute`, só comandos). O primeiro argumento é um comando: texto (`"ls -l"`, `|` encadeia) ou lista (`["wc", "-l"]`). Depois vêm mais comandos ou funções; a saída de uma etapa entra na próxima. Uma função recebe cada linha: `nada` descarta, texto troca, lista vira várias. `[filtro, ".txt"]` chama `filtro(linha, ".txt")`. Não há shell: `*`, `>`, `$` e `&&` são erro (use `["sh", "-c", "…"]`). Último argumento opcional: `mapa([:pasta -> "…", :entrada -> linhas, :ambiente -> mapa([…]), :tempo -> segundos])`.

### `execute(comando, etapa, …)`

Espera terminar. Mapa `:codigo`, `:ok`, `:saida` (linhas), `:erro` (linhas). Código diferente de 0 não é erro: confira `:ok`.

```text
filtro = funcao(linha, oque) { se linha contem oque { linha } }
n = execute("ls", [filtro, ".txt"], ["wc", "-l"]):saida[1]
r = execute("git status --short")
se nao r:ok { escreva_erro(r:erro) }
```

```text
execute(["vim", "notas.txt"], mapa([:terminal -> verdadeiro]))
execute("cargo build", mapa([:terminal -> verdadeiro, :exija -> verdadeiro]))
```

### `linhas_de(comando, etapa, …)`

O comando rodando, lido com `para … em` enquanto as linhas chegam. Sair do laço encerra o comando. A saída de erro vai para a tela.

```text
para linha em linhas_de("tail -f app.log") {
    se linha contem "xxx" { escreva(linha) }
    se linha contem "FIM" { pare }
}
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
