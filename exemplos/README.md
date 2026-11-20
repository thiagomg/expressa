# Exemplos Expressa

Os exemplos são organizados por assunto. As pastas `lib/` e os módulos de apoio
são usados por outros programas. Na raiz do repositório:

```bash
cargo run -- exemplos/linguagem/media.lep
```

Programas que leem o teclado (`texto/saudacao.lep`, `programas/calculadora.lep`,
`escola/bhaskara.lep`, `escola/juros.lep`):

```bash
cargo run -- exemplos/texto/saudacao.lep
```

As calculadoras de `escola/` pedem os dados, avisam sobre
entradas inválidas e terminam após apresentar o resultado. Quando essa opção
é indicada, pressione Enter sem digitar nada para cancelar; o fim da entrada
também encerra. Na configuração padrão, use vírgula para decimais (`2,5`).
Em juros, digite `2` para uma taxa de 2% por período; os valores em reais são
exibidos com duas casas decimais.

Se você nunca programou, comece pelo [tutorial básico](../tutorial/basico.md).
Depois, leia o [tutorial avançado](../tutorial/avancado.md). Para consultar as
funções da linguagem, use o [manual](../docs/manual.md).

As pastas indicam o assunto de cada exemplo. Depois do tutorial, uma sugestão de sequência é: `linguagem/` → `texto/` → `programas/` → `escola/` / `unix/` / `jogos/`.

| Pasta | Papel |
|-------|--------|
| `linguagem/` | recursos da linguagem: laços, funções, importação e tratamento de erros |
| `texto/` | textos, trechos de texto e desenhos com caracteres |
| `programas/` | pequenos programas completos |
| `escola/` | cálculos e exercícios do ensino médio |
| `unix/` | `cat` / `grep` / `wc` |
| `jogos/` | Termo, Forca, Conway, adivinhe o número, tartaruga |
| `lib/` | bibliotecas usadas pelos outros exemplos |
| `tela_grafica/` | desenhos e animações com pixels (`importe "tela_grafica"`) |
| `tarefas/` | lista de tarefas com menu e armazenamento em arquivo |

### `linguagem/`

| Arquivo | O que mostra |
|---------|----------------|
| `media.lep` | laço, `se`, lista, `escreva` |
| `tabuada.lep` | função + `para i em 1..=10` |
| `fatorial.lep` | recursão (`fatorial`, `combinacoes`) |
| `erros.lep` | `se_falhar` (arquivo, divisão, índice) |
| `filtra.lep` | função que recebe função (filtrar / mapear / reduzir) |
| `closures.lep` | função que devolve função |
| `usa_matematica.lep` | `importe "../lib/matematica"` com e sem prefixo para acessar as funções |

`lib/matematica.lep` é a biblioteca (`soma`, `media`, `potencia`, …). `lib/tartaruga.lep` é a tartaruga Logo (`ande`, `vire`, `mostre`, `velocidade`), em cima de `tela_grafica`. `velocidade(t, 0.02)` mostra o traço aos poucos, com `lib/tartaruga.png` (32×32) no lugar da tartaruga.

### `texto/`

| Arquivo | O que mostra |
|---------|----------------|
| `saudacao.lep` | `leia`, `limpe`, `separe`, `maiuscula` |
| `palindromo.lep` | inverte um texto e verifica se ele é igual nos dois sentidos |
| `ascii.lep` | triângulo e gráfico de barras com `repita` |

### `programas/`

| Arquivo | O que mostra |
|---------|----------------|
| `menu_cursor.lep` | guarda o cursor e repete `leia` na mesma linha, com aviso abaixo |
| `diario.lep` | `salve_arquivo`, `adicione_arquivo`, `leia_arquivo` |
| `contatos.lep` | agenda em CSV |
| `calculadora.lep` | lê uma conta e calcula com `avalie` |

`diario.lep` e `contatos.lep` gravam em `programas/saida/` (ignorada pelo git).

### `tela_grafica/`

Tela de pixels (`importe "tela_grafica"`). Execute no terminal ou na Aula (F5) para abrir uma janela. Os exemplos que
chamam `salve` também gravam uma imagem PNG ao lado do programa.

| Arquivo | O que mostra |
|---------|----------------|
| `linha.lep` | linha, círculo, `mostre`, `salve` |
| `texto.lep` | `texto` na caneta (acento, tamanho 16) |
| `casa.lep` | casinha e sol (`cor`, `linha`, `circulo`, `mat::seno`) |
| `clique.lep` | `evento`: clique desenha círculo |
| `gira.lep` | figura andando e girando (`evento()`) |
| `movimento.lep` | laço gráfico a 30 quadros por segundo (FPS), movimento por tempo, setas simultâneas, Shift e espaço |
| `conway_graf.lep` | Jogo da Vida na tela de pixels, com uma figura por célula |
| `sprite.lep` | imagem reutilizável (sprite) com fundo transparente |

### `jogos/`

`explorador.lep` é um exemplo gráfico completo. Setas movem, Shift corre,
R reinicia e Esc termina. Colete os três cristais; `--validar` executa o
roteiro automático e `--30fps` permite comparar com os 60 quadros por segundo (FPS) usados por padrão.
O mapa, a simulação e o roteiro ficam em `jogos/explorador/`.

| Arquivo | O que mostra |
|---------|----------------|
| `adivinhe_o_numero.lep` | adivinhe o número |
| `conway_jogo_da_vida.lep` | Jogo da Vida |
| `explorador.lep` | personagem animado, mapa, câmera, colisões e verificação automática |
| `forca.lep` | Forca |
| `termo.lep` | Termo |
| `tartaruga_quadrado.lep` | Logo: `ande` + `vire(90)` (`importe "../lib/tartaruga"`) |
| `tartaruga_estrela.lep` | Logo: estrela de cinco pontas (`vire(144)`) |

Ângulo 0 olha para cima; `vire` é para a direita; casa no centro da tela.

### `unix/`

| Arquivo | O que mostra |
|---------|----------------|
| `cat.lep` | concatena arquivos |
| `grep.lep` | filtra linhas (`contem`) |
| `wc.lep` | exemplo simples: conta palavras separadas por espaços em branco, ou linhas com `-l` |
| `wc_complexo.lep` | versão completa: linhas, palavras, bytes, caracteres e opções combinadas |
| `lib/linha_cmd.lep` | opções e argumentos da linha de comando |

`wc.lep` mostra um contador simples de palavras, como `wc -w`: pontuação
não separa palavras, mas espaços, tabulações e quebras de linha separam.
Sem arquivos, lê a entrada padrão; `-l` conta quebras de linha. Nos dois exemplos,
os caminhos recebidos são relativos à pasta de onde foram chamados.

No Windows, chame o interpretador; o arquivo `.lep` não precisa de associação
nem da linha `#!/usr/bin/env expressa`. Na raiz do repositório, no PowerShell:

```powershell
.\target\release\expressa.exe exemplos\unix\wc.lep .\meu-arquivo.txt
.\target\release\expressa.exe exemplos\unix\wc.lep -l .\meu-arquivo.txt
"um dois" | .\target\release\expressa.exe exemplos\unix\wc.lep
```

As quebras de linha LF e CRLF são aceitas. O texto é interpretado como UTF-8;
arquivos UTF-16 ou de páginas de código antigas precisam ser convertidos antes.
O PowerShell 7 envia texto em UTF-8 por padrão. No Windows PowerShell 5.1,
configure `$OutputEncoding = [System.Text.UTF8Encoding]::new()` antes de enviar
texto com acentos pelo pipe. Essa configuração afeta o pipe, não a codificação
dos arquivos lidos diretamente pelo exemplo.

`wc_complexo.lep` usa a contagem básica do `wc`: sem opções, mostra quebras de linha,
palavras e bytes. Pontuação não separa palavras; uma última linha sem `\n` não
entra na contagem de linhas. As opções podem ser combinadas (`-lwc`); `-m` conta
caracteres UTF-8. Sem arquivos ou com `-`, lê a entrada padrão. Com vários
arquivos, mostra também o total e continua após erros de leitura, encerrando
com código 1 se algum arquivo falhar. A interpretação de texto cobre os locales
UTF-8 e C/POSIX; outras codificações e extensões como `-L`/`--files0-from` não
fazem parte deste exemplo. A largura das colunas pode diferir da ferramenta
instalada no sistema.

### `escola/`

Contas de caderno com entrada de dados: equação do 2º grau, juros, estatística,
frações, cinemática e MDC/MMC. `seno_cosseno.lep` usa dados fixos que podem
ser alterados no código.
Detalhe em `escola/README.md`.

A raiz quadrada é `raiz(n)` no módulo `mat` (`importe "mat"`). `pi` é um valor do mesmo módulo (`escola/seno_cosseno.lep`).

### `tarefas/`

Execute `tarefas/tarefas.lep` para abrir a lista de tarefas. Digite a letra
inicial destacada de uma opção para criar, concluir, editar ou remover uma tarefa.
`comum.lep`, `menu.lep` e `persiste.lep` são módulos de apoio.
