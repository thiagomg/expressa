# Tutorial Expressa

| | |
|---|---|
| **Este tutorial** | sair do zero (`01`–`08`) |
| [Tutorial avançado](avancado.md) | mapa, texto, módulo, arquivo, tela |
| [Manual](../docs/manual.md) | cada função, com exemplo — não é a especificação |

Você vai fazer a tela escrever, perguntar e decidir. Arquivos da linguagem terminam em `.lep`. As funções e palavras chave da linguagem são em português.

Cada capítulo tem um arquivo em `exemplos/`. Execute, mude uma linha, execute de novo. O desafio no fim é a parte que ensina.

## Como executar

Três formas diferente. Use o que preferir.

**REPL** (conversa: você digita uma linha, ela responde). Muito prático para iniciar e para explorar a linguagem no futuro

```text
expressa
```

Aparece um `>`. Digite `2 + 3` e Enter. Digite `escreva("oi")` e Enter. `sair()` ou `Ctrl+D` encerra.

**Aula** (editor integrado). Novo arquivo (Ctrl+N), cole o código, **Executar** (F5). A saída fica embaixo. Se o programa pedir texto, clique nessa saída, digite, Enter. **Parar** mata um programa que não acaba. Salve como `algo.lep`. F1 abre a ajuda das funções.

**Outro editor** (VS Code, Emacs, Bloco de Notas). Salve `ola.lep` e no terminal:

```text
expressa ola.lep
```

Neste repositório, os arquivos das aulas estão em `tutorial/exemplos/`:

```text
expressa tutorial/exemplos/01_ola.lep
```

Índices começam em **1**: `"Ana"[1] == "A"`.

---

## Explorar no REPL

Não precisa decorar a lista de funções. Abra o REPL (`expressa`) e pergunte.

```text
ajuda                 o que o REPL entende
ajuda linguagem       se, para, função, enquanto…
ajuda funcoes         nomes das nativas (núcleo e módulos)
ajuda escreva         ficha de uma função (assinatura + exemplo)
ajuda raiz
```

`ls` lista o que está visível agora. `ls matriz` (ou `ls tela`, `ls mat`, `ls arquivo`) lista um módulo, mesmo antes do `importe`.

```text
ls
ls mat
importe "mat"
raiz(9)
```

Depois de criar a sua própria função, ela aparece no `ls`:

```text
dobro = função(n) { n * 2 }
ls
```

No expressa-aula, **F1** (ou o botão Ajuda) é o mesmo catálogo. Passe o mouse num nome, ou F1 em cima dele, para a ficha. Digitar já sugere nativas e as funções do arquivo.

`catalogo()` retorna a mesma lista do `ls`, mas de forma crua para ser usada por um script

Setas ↑ ↓ repetem linhas anteriores. `cls` limpa a tela do REPL. `sair` (sem parênteses) encerra; Ctrl+D também.

---

## 1. Escrevendo na tela

Arquivo: `exemplos/01_ola.lep`

```text
escreva("Olá")
escreva("Isso saiu na tela.")
```

`escreva` mostra o que está entre aspas. Cada chamada, uma linha.

No REPL: `escreva("Olá")`

**Mude:** a frase entre aspas. Rode de novo.

**Desafio:** três `escreva`, três frases suas.

---

## 2. Contas

Arquivo: `exemplos/02_contas.lep`

No REPL isto é mais rápido que um arquivo:

```text
2 + 3
10 * 4
35.87 + 12.63
```

Operadores: `+` `-` `*` `/`. No código, decimal com **ponto** (`3.5`). O resultado quando você usar escreva será com **vírgula** (`3,5`) já que esse é o formato em português.

Juntar texto e número:

```text
escreva("2 + 3 = " + (2 + 3))
```

O `+` entre textos junta os dois. Os parênteses na conta fazem a soma primeiro.

**Desafio:** escreva o dobro de 15, na forma `dobro de 15 = 30`.

---

## 3. Nomes

Arquivo: `exemplos/03_nomes.lep`

```text
nome = "Armelina"
idade = 16
escreva("Olá, " + nome)
```

`nome` guarda `"Armelina"`. Na linha seguinte você usa `nome` em vez de escrever Ana de novo. Trocar `nome = "Armelina"` para `"Filadelfo"` muda as duas linhas de baixo.

**Desafio:** um nome `cidade` e um `escreva` que fale a cidade.

---

## 4. Fazendo uma pergunta

Arquivo: `exemplos/04_perguntas.lep`

```text
escreva("Qual o seu nome?")
nome = leia()
escreva("Olá, " + nome)
```

`leia()` espera uma entrada do teclado até que seja pressionado Enter. O que você digitou vai para `nome`.

No expressa-aula: depois de Executar, o cursor vai para a saída. Digite o nome e pressione Enter.

**Desafio:** pergunte também a comida favorita e escreva as duas respostas.

---

## 5. Condições

Arquivo: `exemplos/05_decisao.lep`

```text
nota = 8

se nota >= 7 {
    escreva("Aprovado")
} senão {
    escreva("Reprovado")
}
```

Se a condição for verdadeira, executa o bloco de cima. Senão, o de baixo. `>=` é “maior ou igual”.

Troque `nota = 8` por `5` e execute.

Comparações: `==` (igual), `!=` (diferente), `>` `<` `>=` `<=`.

Dá para escrever o mesmo com `inicio` e `fim` no lugar de `{` e `}`. Os dois valem; não misture os dois estilos no mesmo bloco.

**Desafio:** se a nota for 10, escreva `"Gabaritou"` (use `ou se nota == 10` no meio).

---

## 6. Usando listas

Arquivo: `exemplos/06_lista.lep`

```text
notas = [7, 8, 6.5, 9]
escreva("primeira: " + notas[1])
escreva("quantidade: " + tamanho(notas))
```

`notas[1]` retorna o primeiro item. `tamanho` retorna a quantidade.

Para **cada** nota, some:

```text
soma = 0
para nota em notas {
    soma = soma + nota
}
escreva("média: " + (soma / tamanho(notas)))
```

`para nota em notas` pega um item por vez, chama de `nota` e executa o bloco.

Versão com “Aprovado/Reprovado”: `../exemplos/linguagem/media.lep`.

**Desafio:** acrescente uma quinta nota na lista. A média muda sozinha.

---

## 7. Usando funções

Arquivo: `exemplos/07_funcao.lep`

A tabuada do 7 e a do 3 são semelhantes, só muda o número. Em vez de copiar o laço duas vezes:

```text
tabuada = função(n) {
    para i de 1 até 10 {
        escreva(n + " x " + i + " = " + (n * i))
    }
}

tabuada(7)
tabuada(3)
```

`n` é o número que você passa entre parênteses. `para i de 1 até 10` conta 1, 2, … 10.

O mesmo programa está em `../exemplos/linguagem/tabuada.lep` (lá usa `funcao` e `inicio`/`fim` — mesma coisa).

**Desafio:** `tabuada(9)`.

No REPL, depois de colar a função: `ls` — `tabuada` aparece na lista. `ajuda tamanho` mostra uma nativa no mesmo estilo.

---

## 8. Desenhar

Arquivo: `exemplos/08_desenho.lep`

```text
repete = função(pedaco, n) {
    s = ""
    repita n vezes {
        s = s + pedaco
    }
    s
}

para i de 1 até 6 {
    escreva(repete("*", i))
}
```

`repita n vezes` repete o bloco n vezes. `repete("*", 3)` vira `"***"`. O `para` faz o `i` mudar de 1 até 6:

```text
*
**
***
****
*****
******
```

Mais desenhos (triângulo centrado, histograma): `../exemplos/texto/ascii.lep`.

**Desafio:** um retângulo 4×8 de `#`, ou o triângulo invertido (6 estrelas, depois 5, …).

---

## 9. Um programa de verdade

Agora junte o que você já viu. Não precisa escrever do zero: abra, execute, mude um número.

**Caixa da padaria** — `../exemplos/programas/caixa.lep`

Lista de compras, preço de cada item, desconto se passar de R$ 20, recibo na tela. Funções (`preco_de`, `subtotal`, `recibo`) iguais às da tabuada: nome + parênteses.

Mude o `pedido` no fim do arquivo e execute.

**Adivinhe o número** — `../exemplos/jogos/adivinhe_o_numero.lep`

Número de 1 a 100, você tenta, o programa diz maior ou menor. `enquanto` repete até acertar ou Enter vazio. `aleatório` vem de `importe "mat"`. `pinte(:verde)` colora a mensagem.

No expressa-aula, cada palpite é uma linha na saída. **Parar** se quiser desistir no meio.

**Forca** — `../exemplos/jogos/forca.lep`

Uma letra por vez, teclado colorido, boneco em ASCII. No fim do arquivo tem a lista `palavras`. Troque as palavras. Ou passe uma na linha de comando:

```text
expressa exemplos/jogos/forca.lep ESCOLA
```

No expressa-aula, a caixa **argumentos** em cima é a mesma coisa.

---

## Se der erro

A mensagem aponta o arquivo e a linha. No expressa-aula, clique nela para pular para o sítio.

Erros comuns no começo:

- aspas só de um lado: `"Olá`
- `escreva(Olá)` sem aspas — a linguagem acha que `Olá` é um nome
- `se nota > 7` sem `{` ou `inicio` no bloco
- `notas[0]` — não existe; o primeiro é `notas[1]`

---

## Depois

| O que | Onde |
|--------|------|
| texto (fatia, maiúscula) | `../exemplos/texto/poema.lep` |
| boletim | `../exemplos/programas/turma.lep` |
| gravar arquivo | `../exemplos/programas/diario.lep` |
| conta de caderno (Bhaskara, juros) | `../exemplos/escola/` |
| Termo, Conway, desenhos de bicho | `../exemplos/jogos/` |
| lista de funções | REPL: `ajuda` / `ls` / `ajuda escreva` · Aula: **F1** · [manual](../docs/manual.md) |

A folha de consulta é um super resumo, não um capítulo. Use quando esquecer um nome (`tamanho`, `contem`, `para`).

Quando `01`–`08` e um dos programas da aula 9 estiverem ok: [tutorial avançado](avancado.md).
