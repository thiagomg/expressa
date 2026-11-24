# Expressa — folha de consulta

Arquivos `.lep`. Palavras em português. Índices começam em **0**.

Tutorial: [básico](tutoriais/basico.md). Avançado: [avançado](tutoriais/avancado.md).\
Manual: `docs/manual.md`. Spec: `docs/especificacao-linguagem.md`. Fichas: `docs/nativas/*.lep`.

```text
expressa                      REPL
expressa prog.lep [args…]
expressa debug prog.lep
expressa --numeros en         texto 1,000.5
expressa --aritmetica float   IEEE-754
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
| `numero` | `10` `3.14` `-8` (no código, ponto; valor decimal exato) |
| `texto` | `"olá"` |
| `bool` | `verdadeiro` `falso` |
| lista | `[1, 2, 3]` |
| par | `:nome -> "Ana"` |
| mapa | `mapa([:nome -> "Ana"])` |
| matriz | `matriz([[1, 2], [3, 4]])` só números; `importe "matriz"` |
| conjunto | `conjunto([:ana, 1])` valores únicos |
| intervalo | `0..3` `1..=3` `5..` inteiros gerados sob demanda |
| `funcao` | `funcao(x) { x + 1 }` (também `função`) |
| `nada` | `nada` |

Também existem `imagem` (`importe "imagem"`), `modulo` e `processo` (`importe "processo"`). `tipo(x)` devolve esses nomes, sem acento.

`nada` representa ausência de valor. Pode ser atribuído, comparado e retornado explicitamente. `escreva` e gravar arquivo também devolvem `nada`.

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

Funções **leem** nomes de fora, mas **não os reatribuem**. Elementos de listas, mapas e matrizes compartilhados podem ser alterados.

```text
// uma linha
/* várias linhas */
#!/usr/bin/env expressa        // só na 1ª linha; # não é comentário
```

`inicio` fecha com `fim`; `{` fecha com `}`. Não misture.

Um comando por linha. A quebra de linha só é livre dentro de `( )` e `[ ]`: para quebrar uma conta, use parênteses. Operador no fim ou no começo da linha é erro. Valor calculado e não usado (`x + 1` sozinho, fora da última linha do bloco) é erro.

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
(0..10) contem 3               // intervalo: o número é gerado?
```

```text
mat::soma(1, 2)                // nome no módulo
pessoa:nome                    // chave de mapa (= pessoa["nome"])
xs.tamanho()                   // igual a tamanho(xs)  — o () é obrigatório
```

---

## Controle

```text
r = se nota >= 7 { "aprovado" } senao { "reprovado" }   // como valor, senao obrigatório
se n < 0 { escreva("negativo") }                    // comando: senao opcional

retorne verdadeiro                 // só numa função; sai na hora
retorne                            // devolve nada

repita 3 vezes { escreva("olá") }

para i em 1..=5 { escreva(i) }          // 1, 2, 3, 4, 5
para i em 0..tamanho(xs) { escreva(xs[i]) }
para nome em ["Ana", "Bruno"] { escreva(nome) }
para ch em "olá" { escreva(ch) }    // texto: um caractere por vez
para p em pessoa { escreva(p:chave, p:valor) }   // mapa: pares na ordem da impressão
pare                               // sai do laço
continue                           // próxima volta

i += 1                         // números: += e -=
nome += " Silva"               // texto: só +=

i = 1
enquanto i <= 3 {
    escreva(i)
    i = i + 1
}

valor = 10 / 0 se_falhar { 0 }
importe "arquivo"
linhas = leia_arquivo("x.txt") se_falhar { [] }
garanta(n > 0)                 // erro: condição incorreta n > 0
se n < 0 { falhe("negativo") }
```

A captura opcional de `se_falhar` recebe a mensagem do erro como texto e só existe no bloco:

```text
valor = numero("abc") se_falhar erro {
    escreva("Falha:", erro)
    falso
}
```

O bloco é obrigatório, inclusive sem captura: `numero("abc") se_falhar { falso }`.
Erros no tratamento se propagam normalmente.

Sem `se_falhar`, um erro encerra o programa.

---

## Funções

Última expressão do bloco é o resultado. `retorne` sai mais cedo.

```text
soma = funcao(x, y) { x + y }
escreva(soma(10, 5))           // 15

avalie("2 + 3 * 4")           // 14  (usa x, funções… do escopo atual)
escreva(avalie(leia("> ")) se_falhar { "conta inválida" })
modulos()                      // imprime módulos, incluindo arquivos importados
funcoes()  funcoes("matriz")    // imprime funções e documentação
modulos(:dados)                // mapas com :nome e :funcoes
funcoes(nada, :dados)             // mapas com :nome, :args e :doc
ajuda("escreva")  ajuda("matriz::zeros")

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
xs[0]                          // 10
xs[1..3]                       // [20, 30]  (teto exclusivo)
xs[1..]                        // até o fim
"b"[0..2]                      // "b" (não falha)
xs + [50]
xs += [50]                     // xs = xs + [50]
xs = xs.remova(2)               // sem o índice 2
xs = xs.remova(2..4)            // sem 2 e 3
tamanho(xs)  primeiro(xs)  ultimo(xs)   // também texto: primeiro("olá") é "o"
0..3  1..=2  ..3               // intervalo: gera 0, 1, 2
eh_infinito(0..)               // verdadeiro; tamanho(0..) é erro

t = "  Maria Silva  "
t[0]  t[0..4]
t[1] = "x"                     // um caractere; reatribui t
tamanho(t)  maiuscula(t)  minuscula(t)  limpe(t)  sem_acento(t)   // também maiúscula, minúscula
substitua(t, "Maria", "Ana")
procure("onde", "n")          // 1;  nada se não achar
procure([10, 20, 30], 20)     // 1
separe("a,b,c", ",")           // ["a", "b", "c"]
junte(["a", "b"], " - ")
formate("{:<8} {:>5}", "Ana", 10)  // colunas: < esquerda  > direita  ^ centro
                                   // mapa: campo :_como_texto (função) vira o {}

:nome                          // o texto "nome"
p = :nome -> "Ana"
p:chave  p:valor
pessoa = mapa([:nome -> "Ana", :idade -> 25])
pessoa:nome  pessoa[:nome]  pessoa["nome"]
pessoa:nome = "Bia"
pessoa["cidade"] = "Fortaleza"
pessoa = pessoa.remova("idade")
pessoa += :idade -> 26          // mapa novo com a chave
chaves_de(pessoa)  valores_de(pessoa)   // listas na ordem da impressão
como_texto(2.5)                // "2,5": o texto que escreva mostraria

s = conjunto([:ana, :bia])
s contem :ana
s += :carlos
s = s.remova(:bia)

importe "matriz"
A = matriz([[1, 2, 3], [4, 5, 6]])
A[0, 1]                        // 2
A[0]                           // linha → lista
A.xy(x, y)                     // A[y, x]; base zero
A.xy(x, y, valor)        // altera A[y, x]; devolve A
A + B    3 * A    A * B
transposta(A)  det(A)  identidade(3)
zeros(2, 5)  uns(3)  cheia(2, 3, 7)
linhas(A)  colunas(A)          // 2 e 3
tamanho(A)                     // também linhas
```

---

## Entrada e saída

```text
escreva("Média:", 7.5)         // stdout; [7.5] dentro de lista
escreva_erro("falhou")         // stderr (não entra em > arquivo)
importe "sistema"
durma(1)                       // espera 1 segundo  (aceita 0.5)
saia()                         // encerra, código 0
saia(1)                        // encerra com falha (não pega se_falhar)

nome = leia("Seu nome: ")      // uma linha, sem o Enter
idade = numero(leia("Idade: ")) se_falhar { 0 }    // também número()
linhas = leia_linhas()         // resto da entrada → lista; vazio = []

busca = argumentos[0]          // depois do .lep; REPL → []
tipo(10)                       // "numero"  (texto, bool, lista, mapa, …)
tipos()                        // lista desses nomes

importe "tela"
limpe_tela()                   // limpa a tela
posicao_cursor(0, 0, falso)     // cursor no canto sem apagar nem fazer flush
posicao_cursor(linha, coluna)   // move e faz flush (mostre = verdadeiro)
quadro(tela)                   // cursor no canto + texto; apaga resto da linha e embaixo
nova_linha                     // "\n" no Unix, "\r\n" no Windows
tamanho_tela()                 // mapa :linhas, :colunas (24×80 sem TTY)
b = bloco(0, 27)               // mapa com :linha e :coluna
b.escreva_em("Q W E R T")      // uma linha na coluna; devolve b
eh_terminal()                  // teclado?  também é_terminal()
"HP".pinte(:verde)             // letra; :branco, :vermelho = letra e fundo
"    ".fundo(:azul)            // só o papel
"GO".negrito()
// cores: normal preto vermelho verde amarelo azul magenta ciano branco
```

```text
define_opcao(:formato_numero, :pt)                  // 1.000,5   (padrão de texto)
define_opcao(:formato_numero, :en)                  // 1,000.5
define_opcao(:tipo_numero, :decimal)             // 0.1+0.2 = 0.3 (padrão)
define_opcao(:tipo_numero, :float)               // IEEE-754, como Python
define_opcao(:fechar_janelas, verdadeiro)         // fechar ao terminar
numero("3,14")                 // só o formato ativo: em pt, "3.14" é erro; também número()

importe "mat"
aleatorio(1, 6)                // inteiro inclusive; também aleatório()
semente(1)                     // mesma sequência de aleatorio
raiz(9)                        // 3
pi                             // valor (não pi())
seno(radianos(30))             // 0,5; também sen, cosseno/cos, tangente
graus(pi)                      // 180
log10(1000)                    // 3; log é o natural
potencia(2, 3)                 // 8; também potência
piso(3.7)  teto(3.1)  arredonde(1.5)
```

Literais no código sempre com ponto: `3.14`.

```text
importe "arquivo"
linhas = leia_arquivo("dados.txt") se_falhar { [] }
salve_arquivo("saida.txt", linhas)
adicione_arquivo("saida.txt", ["nova"])

dados = leia_csv("pessoas.csv")    // lista de listas de texto
salve_csv("saida.csv", dados)
```

```text
importe "mat"
cart = importe "cartesiano"
c = cart::documento()
g = cart::grafico()
g.cart::eixos(-2 * pi, 2 * pi, -1.5, 1.5)
g.cart::plote(função(x) { seno(x) }, :azul)
c.cart::adicione(g)
c.cart::mostre()                 // mesma janela a cada mostre; não espera
```

```text
tela_grafica = importe "tela_grafica"
g = tela_grafica::nova(320, 200) // pixels; (0,0) canto superior esquerdo
g = tela_grafica::nova(160, 144, 4) // janela 640×576; canvas 160×144
escreva(tela_grafica::cores()) // nomes das cores predefinidas
laranja = tela_grafica::rgb(255, 128, 0) // "#FF8000"; inteiros de 0 a 255
g.tela_grafica::cor(laranja) // cor e fundo também aceitam "#RRGGBB"
g.tela_grafica::cor(:azul)
g.tela_grafica::linha(10, 10, 310, 190)
g.tela_grafica::circulo(160, 100, 40)
g.tela_grafica::retangulo(10, 10, 40, 30) // preenchido, cantos inclusivos
g.tela_grafica::fundo(:transparente)
sprite = g.tela_grafica::como_imagem()
g.tela_grafica::imagem(sprite, 0, 0) // canto superior esquerdo
g.tela_grafica::imagem(sprite, 160, 100, mapa([:eixo -> mapa([:x -> 16, :y -> 16]), :angulo -> 90]))
g.tela_grafica::texto(8, 8, "olá")  // canto superior esquerdo; tamanho 8
g.tela_grafica::mostre()         // janela irmã; o próximo mostre atualiza
ev = g.tela_grafica::evento() // captura sem esperar interação
se ev:teclas contem :direita { x += 3 } // estado atual
se ev:apertadas contem :espaco { pule() } // novos pressionamentos
// ev:soltas, ev:eventos (em ordem), ev:sair; modificadores: shift/control/alt/super
g.tela_grafica::apague()        // zera o desenho
g.tela_grafica::salve("casa.png")
escreva(g.tela_grafica::pixel(10, 10))
```

```text
l = g.tela_grafica::laço_gráfico()
config = mapa([
    :fps -> 30,
    :eventos -> eventos, // função(ev, dt): entrada e movimento; dt em segundos
    :desenho -> desenhe, // função(g): o laço apaga antes e apresenta depois
])
l.tela_grafica::execute(config) // inicia e espera terminar
// Dentro de um callback: l.tela_grafica::pare()
```

```text
importe "processo"               // só na linha de comando
r = execute("ls", [filtro, ".txt"], ["wc", "-l"])   // filtro(linha, ".txt")
r:saida  r:erro  r:codigo  r:ok  // linhas, linhas, número, bool
execute("sort", mapa([:entrada -> nomes, :pasta -> "dados", :tempo -> 5]))
execute("make", mapa([:exija -> verdadeiro]))       // código != 0 é erro
execute(["vim", "a.txt"], mapa([:terminal -> verdadeiro]))   // teclado e tela
para l em linhas_de("tail -f app.log") { se l contem "FIM" { pare } }
```

Caminhos relativos: pasta do `.lep`, não a pasta de onde você chamou `expressa`. `~` e `~/pasta` são a pasta pessoal.

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

Nativos: `sistema`, `matriz`, `arquivo`, `tela`, `mat`, `crono`, `cartesiano`, `tela_grafica`, `imagem`, `processo`. Nome nu sem `/` nem `.lep` procura o nativo; se existir também `matriz.lep` no mesmo diretório, `importe "matriz"` é erro. Nomes não colidem: um `importe` achatado com nome que já existe (do programa, do núcleo ou de outro módulo, como `fundo` ou `adicione`) é erro, e reatribuir um nome importado também (`importe "mat"` e depois `raiz = 1`); use apelido. `importe "tela_gráfica"` também vale. Nativas com palavra acentuada aceitam as duas grafias (`maiuscula`/`maiúscula`).

---

## Nativas (resumo)

Núcleo (sempre no escopo):

| | |
|---|---|
| `escreva` `escreva_erro` `saia` `falhe` `garanta` | imprimir / encerrar / erro |
| `leia` `leia_linhas` | teclado / pipe |
| `argumentos` | lista (não é função) |
| `numero` `número` `formato` `avalie` `modulos` `funcoes` `ajuda` | número / executar texto / listar / ficha |
| `tipo` `tipos` | nome do tipo |
| `mapa` `conjunto` `clone` | construtores / cópia profunda |
| `tamanho` `primeiro` `ultimo` `eh_infinito` `chaves_de` `valores_de` | coleção / intervalo / mapa |
| `maiuscula` `minuscula` `sem_acento` `remova` `substitua` `procure` `separe` `junte` `limpe` `formate` `como_texto` | texto / lista / mapa / conjunto |
| `ordene` `comeca_com` `termina_com` | ordem; começo e fim de texto |

Módulos (`importe "…"`):

| | |
|---|---|
| `tela` | `limpe_tela` `posicao_cursor` `apague_linha` `eh_terminal` `pinte` `fundo` `negrito` `tamanho_tela` `quadro` `bloco` `escreva_em` `nova_linha` |
| `mat` | `pi` `raiz` `aleatorio` `semente` `seno` `cosseno` `tangente` `arcosseno` `arcocosseno` `arcotangente` `arcotangente2` `radianos` `graus` `exp` `log` `log10` `potencia` `piso` `teto` `arredonde` |
| `matriz` | `matriz` `transposta` `det` `identidade` `zeros` `uns` `cheia` `linhas` `colunas` `xy` |
| `arquivo` | `leia_arquivo` `salve_arquivo` `adicione_arquivo` `leia_csv` `salve_csv` `existe` `eh_pasta` `liste` `crie_pasta` `apague_arquivo` `apague_pasta` `mova` `copie` `junte_caminho` `nome_do_arquivo` `pasta_de` `extensao` `pasta_atual` `leia_bytes` `texto_utf8` |
| `sistema` | `durma` `ambiente` |
| `crono` | `agora` `hoje` `data` `adicione` `dias_entre` `eh_bissexto` `formate_data` |
| `cartesiano` | `documento` `grafico` `eixos` `plote` `adicione` `mostre` |
| `tela_grafica` | `nova` `cor` `fundo` `linha` `circulo` `ponto` `retangulo` `texto` `imagem` `apague` `pixel` `como_imagem` `salve` `mostre` `evento` `laco_grafico` `execute` `pare_laco` |
| `imagem` | `carregue` `salve` `rotacione` `redimensione` `largura` `altura` |
| `processo` | `execute` `linhas_de` (só na linha de comando) |

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
modulos()    funcoes()    funcoes("matriz")    ajuda("escreva")
limpe_tela                // comando do REPL: limpa a tela (também cls)
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
busca = argumentos[0] se_falhar { "" }
para linha em leia_linhas() {
    se linha contem busca { escreva(linha) }
}
```

```text
cat nomes.txt | expressa grep.lep Thiago
```
