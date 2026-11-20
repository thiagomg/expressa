# Tutorial básico

| Material | O que você encontra |
|---|---|
| [Manual](../docs/manual.md) | referência da linguagem e das funções |
| **Tutorial básico** | primeiros passos, capítulos 1 a 8 e projetos |
| [Tutorial avançado](avancado.md) | coleções, funções, módulos, arquivos e gráficos |

Neste tutorial, você vai criar programas que mostram mensagens, fazem perguntas, calculam e tomam decisões. Os arquivos da linguagem têm a extensão `.lep`. As funções e as palavras-chave da Expressa são em português.

Cada capítulo tem um programa de exemplo na pasta `tutorial/exemplos/`. Caminhos como `exemplos/01_ola.lep` partem da pasta `tutorial/`; caminhos como `../exemplos/jogos/forca.lep` apontam para a pasta de exemplos, ao lado de `tutorial/`. Execute o programa, mude uma linha e execute de novo para observar o resultado. Se a pasta for somente leitura, como na pasta de instalação, salve uma cópia em outra pasta antes de mudar. No fim de cada capítulo, há um desafio para você praticar.

## Como executar

Você pode experimentar a linguagem de três formas. Escolha a que preferir.

> Para executar `expressa` (`expressa.exe` no Windows) de qualquer pasta no terminal, a pasta do executável precisa estar no `PATH`, a lista de pastas em que o sistema procura programas.

**REPL** é o modo interativo: você digita uma expressão ou um comando e vê o resultado em seguida. Ele é útil para fazer contas, testar funções e explorar a linguagem. Para abrir, digite no terminal:

```text
expressa
```

Quando aparecer o símbolo de entrada, digite `2 + 3` e pressione Enter. Depois, experimente `escreva("oi")`. Para encerrar, digite `sair` (sem parênteses) ou pressione Ctrl+D.

**expressa-aula** é o editor integrado da Expressa. Crie um arquivo com Ctrl+N, digite ou cole o código e clique em **Executar** (F5). O resultado aparece no painel de saída, na parte inferior da janela. Se o programa pedir uma resposta, digite nesse painel e pressione Enter. Use **Parar** para interromper a execução. Salve o arquivo com a extensão `.lep`, como `algo.lep`. A tecla F1 abre a ajuda das funções.

**Outro editor**, como VS Code, Emacs ou Bloco de Notas, também pode ser usado. Salve o código em `ola.lep` e execute no terminal, na pasta em que você salvou o arquivo:

```text
expressa ola.lep
```

Na pasta que contém `tutorial/`, execute um capítulo assim:

```text
expressa tutorial/exemplos/01_ola.lep
```

---

## Explorar no REPL

Não precisa decorar a lista de funções. Abra o REPL (`expressa`) e pergunte.

```text
modulos()             // módulos disponíveis, incluindo o núcleo
funcoes()             // funções visíveis na sessão
funcoes("mat")        // funções de um módulo, mesmo antes de importar
ajuda("escreva")      // ficha de uma função
ajuda("mat::raiz")
```

Depois de criar a sua própria função, ela aparece em `funcoes()`:

```text
dobro = função(n) { n * 2 }
funcoes()
```

No expressa-aula, **F1** ou o botão **Ajuda** abre o catálogo de funções. Para consultar uma função específica, passe o mouse sobre o nome dela ou posicione o cursor nesse nome e pressione F1. Enquanto você digita, o editor sugere funções da linguagem e funções definidas no arquivo.

`funcoes(nada, :dados)` retorna uma lista de mapas com nome, argumentos e documentação. `modulos(:dados)` retorna os módulos com suas listas de funções. Essas consultas não imprimem e podem ser usadas no próprio programa.

As setas ↑ e ↓ permitem consultar e reutilizar comandos anteriores. `limpe_tela` (ou `cls`) limpa a tela do REPL. `sair` (sem parênteses) encerra; Ctrl+D também.

---

## 1. Escrevendo na tela

Arquivo: `exemplos/01_ola.lep`

```text
escreva("Olá")
escreva("Isso saiu na tela.")
```

`escreva` mostra o texto que está entre aspas. Cada chamada escreve uma linha.

No REPL: `escreva("Olá")`

**Mude:** a frase entre aspas. Execute de novo.

**Desafio:** use três chamadas de `escreva` para mostrar três frases suas.

---

## 2. Contas

Arquivo: `exemplos/02_contas.lep`

Você pode fazer estas contas diretamente no REPL:

```text
2 + 3
10 * 4
35.87 + 12.63
```

Use `+` para somar, `-` para subtrair, `*` para multiplicar e `/` para dividir. No código, escreva números decimais com **ponto**, como `3.5`. Por padrão, `escreva` mostra esses números com **vírgula**, como `3,5`, seguindo a convenção do português brasileiro.

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
escreva(nome + " tem " + idade + " anos")
```

`nome` é uma variável: um nome associado a um valor, neste caso `"Armelina"`. Nas chamadas de `escreva`, você usa a variável em vez de repetir esse texto. Se trocar a primeira linha por `nome = "Filadelfo"`, as duas mensagens passam a usar o novo nome.

**Desafio:** crie uma variável chamada `cidade` e use `escreva` para mostrar o valor dela.

---

## 4. Fazendo uma pergunta

Arquivo: `exemplos/04_perguntas.lep`

```text
escreva("Qual o seu nome?")
nome = leia()
escreva("Olá, " + nome)
```

`leia()` espera você digitar uma resposta e pressionar Enter. O texto digitado fica guardado na variável `nome`.

No expressa-aula: depois de Executar, o cursor vai para a saída. Digite o nome e pressione Enter. No terminal (`expressa tutorial/exemplos/04_perguntas.lep`) o programa também espera o Enter.

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

escreva("nota: " + nota)
```

Se a condição for verdadeira, o programa executa o bloco de cima. Caso contrário, executa o bloco de baixo. `>=` é “maior ou igual”.

Troque `nota = 8` por `5` e execute.

Comparações: `==` (igual), `!=` (diferente), `>` `<` `>=` `<=`.

Dá para escrever o mesmo com `inicio` e `fim` no lugar de `{` e `}`. Os dois valem; não misture os dois estilos no mesmo bloco.

**Desafio:** faça o programa escrever `"Gabaritou"` quando a nota for 10. Comece com `se nota == 10`, use `ou se nota >= 7` para as outras notas aprovadas e deixe `senão` para as reprovadas. A ordem importa: o programa executa apenas o primeiro bloco cuja condição for verdadeira.

---

## 6. Usando listas

Arquivo: `exemplos/06_lista.lep`

```text
notas = [7, 8, 6.5, 9]
escreva("primeira: " + notas[0])
escreva("quantidade: " + tamanho(notas))
```

`notas[0]` retorna o primeiro item: os índices começam em **0**. Com 4 notas, o último índice é 3. `tamanho` retorna a quantidade.

Para calcular a média, percorra a lista e some todas as notas:

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

A tabuada do 7 e a do 3 seguem o mesmo procedimento; só muda o número. Uma função permite escrever esse procedimento uma vez e usá-lo com números diferentes:

```text
tabuada = função(n) {
    para i em 1..=10 {
        escreva(n + " x " + i + " = " + (n * i))
    }
}

tabuada(7)
tabuada(3)
```

`n` é o número que você passa entre parênteses. `para i em 1..=10` conta 1, 2, … 10. Com `..=`, o último número entra; com `..`, ele fica de fora: `1..10` conta de 1 a 9.

O mesmo programa está em `../exemplos/linguagem/tabuada.lep` (lá usa `funcao` e `inicio`/`fim` — mesma coisa).

**Desafio:** acrescente uma chamada a `tabuada(9)` e execute o programa.

No REPL, digite `funcoes()` depois de definir a função: `tabuada` aparece na lista. Para consultar uma função da linguagem, experimente `ajuda("tamanho")`.

---

## 8. Desenhando com texto

Arquivo: `exemplos/08_desenho.lep`

```text
repete = função(pedaco, n) {
    s = ""
    repita n vezes {
        s = s + pedaco
    }
    s
}

para i em 1..=6 {
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

**Desafio:** desenhe um retângulo com 4 linhas e 8 colunas de `#`, ou inverta o triângulo: comece com 6 estrelas, depois 5, e assim por diante.

---

## Projetos para praticar

Escolha um dos programas abaixo para praticar o que aprendeu. Abra o arquivo, execute e experimente mudar um valor ou uma mensagem.

**Palíndromo** — `../exemplos/texto/palindromo.lep`

O programa verifica se uma frase pode ser lida da mesma forma nos dois sentidos, como "arara". As funções `inverta`, `so_letras` e `eh_palindromo` são chamadas da mesma forma que `tabuada`, com o nome seguido de parênteses, e usam `para`, `se` e textos.

Troque as frases das chamadas de `mostra` no fim do arquivo e execute.

**Adivinhe o número** — `../exemplos/jogos/adivinhe_o_numero.lep`

O programa sorteia um número de 1 a 100. A cada tentativa, ele informa se você precisa escolher um número maior ou menor. O laço `enquanto` repete as tentativas até você acertar ou pressionar Enter sem digitar nada. A função `aleatório` pertence ao módulo `mat`; `pinte(:verde)` deixa a mensagem verde.

No expressa-aula, digite cada palpite no painel de saída. Use **Parar** se quiser interromper a partida.

**Forca** — `../exemplos/jogos/forca.lep`

Uma letra por vez, teclado colorido, boneco em ASCII. No fim do arquivo tem a lista `palavras`. Troque as palavras. Ou passe uma na linha de comando:

```text
expressa exemplos/jogos/forca.lep ESCOLA
```

Esse comando parte da pasta que contém `exemplos/`.

No expressa-aula, use o campo **Argumentos** para passar a palavra ao programa.

---

## Se der erro

A mensagem de erro indica o arquivo e a linha em que o problema ocorreu. No expressa-aula, clique na mensagem para ir até essa linha.

Erros comuns no começo:

- aspas só de um lado: `"Olá`
- `escreva(Olá)` sem aspas — a linguagem acha que `Olá` é um nome
- `se nota > 7` sem `{` ou `inicio` no bloco
- `notas[-1]` — não existe; o primeiro é `notas[0]`
- uma conta dividida em duas linhas, como `total = 10 +` e `5` na linha de baixo — a linha termina o comando; para quebrar uma conta, coloque-a entre parênteses
- dois comandos na mesma linha — escreva um comando por linha

---

## Depois

| O que | Onde |
|--------|------|
| texto (separe, maiúscula) | `../exemplos/texto/saudacao.lep` |
| média, mediana e moda | `../exemplos/escola/estatistica.lep` |
| gravar arquivo | `../exemplos/programas/diario.lep` |
| conta de caderno (Bhaskara, juros) | `../exemplos/escola/` |
| Termo, Forca, Conway, tartaruga | `../exemplos/jogos/` |
| lista de funções | REPL: `modulos()` / `funcoes()` / `ajuda("escreva")` · expressa-aula: **F1** · [manual](../docs/manual.md) |

A [folha de consulta](../docs/folha-de-consulta.md) reúne a sintaxe e os nomes mais usados. Consulte quando precisar lembrar como usar `tamanho`, `contem` ou `para`, por exemplo.

Depois de concluir os capítulos 1 a 8 e experimentar um dos projetos, siga para o [tutorial avançado](avancado.md).
