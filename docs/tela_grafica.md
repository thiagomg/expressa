# Tela gráfica: janela, mensagens e `evento`

A VM Expressa **não abre janela**. Ela monta um mapa (`TelaGrafica`: tamanho, fundo, lista de ops) e fala com um **host**. Quem pinta e ouve o mouse é o processo `expressa-grafico` (CLI) ou o GTK da Aula.

`mostre()` não espera. `evento()` / `evento(segundos)` espera um recado de volta. `salve` / `pixel` rasterizam **na VM**, sem o visor.

## O que o aluno escreve

```
tela_grafica = importe "tela_grafica"
imagem = importe "imagem"

g = tela_grafica::nova(320, 200)
nave = imagem::carregue("nave.png")

angulo = 0
x = 40
y = 100

enquanto verdadeiro {
    g.apague()
    g.imagem(nave, x, y, angulo)
    g.mostre()
    ev = g.evento(0.05)
    se ev:tipo == :sair { pare }
    se ev:tipo == :tecla e ev:tecla == "q" { pare }
    se ev:tipo == :clique {
        // ev:x ev:y em pixels da tela (origem canto superior esquerdo)
    }
    angulo += 6
    x += 2
}
```

`carregue` lê o PNG na VM (sandbox, pasta do `.lep`). `nova` não abre janela. O handle da figura é do **protocolo**, não da linguagem: o aluno continua com `Value::Imagem`.

`g.evento()` espera até clique, tecla ou fechar. `g.evento(0.05)` espera até 50 ms. `g.evento(0)` olha uma vez. Sem janela: `se_falhar`.

Mapa devolvido:

| chave | valores |
|---|---|
| `:tipo` | `clique`, `tecla`, `sair`, `nada` (`solte` / `mova` depois) |
| `:x`, `:y` | pixel, y para baixo, 0-based (já convertidos da janela, que escala) |
| `:botao` | `esquerdo`, `direito`, `meio`, `nenhum` |
| `:tecla` | `"q"`, `"Enter"`, `"Esc"`, `" "`, `"Cima"`, … |

Dois `nova()` são duas janelas; `g.evento()` escuta a de `g`. Parar na Aula desbloqueia como o `leia` (evento `:sair`).

## Transporte

Mensagens com **prefixo de tamanho** (`u32` little-endian + payload). Stream nos pipes stdin/stdout **do filho**, não no terminal do aluno (`leia` continua no stdin do programa). Sem nng: o 64 KiB do pipe é buffer, não teto de recado.

Um filho por `nova()`. A Aula usa os mesmos recados em `mpsc` (a VM já corre numa thread; o GTK pinta).

`MOSTRE` **não** espera ACK. `ESPERE` é o único pedido que tem resposta (`EVENTO`).

## Recados

VM → visor:

| recado | quando |
|---|---|
| `ABRA id largura altura fundo` | primeiro `mostre` (ou `evento`) da instância |
| `CARREGUE_FIGURA handle largura altura rgba` | primeira vez que aquele bitmap (mesmo `Arc` de pixels) aparece nas ops |
| `MOSTRE id [ops…]` | cada `mostre`; ops de imagem citam `handle`, sem RGBA |
| `ESPERE id timeout_ms` | cada `evento`; `-1` até acontecer, `0` olha e segue, `50` = 50 ms |

Visor → VM:

| recado | quando |
|---|---|
| `EVENTO id tipo x y botao tecla` | resposta de `ESPERE` |

O **handle** é um `u32` escolhido pela VM (contador, identidade = ponteiro do `Arc` de pixels). O visor não escolhe id. `CARREGUE` também não espera ACK: no mesmo pipe, `CARREGUE` imediatamente antes do `MOSTRE` que usa o handle chega em ordem.

Ops no `MOSTRE`: `ponto`, `linha`, `circulo` (cor pelo nome, inclusive `transparente`), `retangulo x1 y1 x2 y2` (preenchido, cantos inclusivos), `texto x y tamanho cor t` e `imagem handle x y angulo` (graus, horário, 0 = topo para cima). O visor preenche o fundo (transparente na janela vira papel branco), replay das ops, apresenta. `apague` no Expressa esvazia `:ops`; o handle da figura **continua** válido. `como_imagem` rasteriza na VM (RGBA) e não vai no pipe.

## Fluxo: abrir figura, girar, andar

```mermaid
sequenceDiagram
    participant Aluno
    participant VM
    participant Pipe
    participant Visor

    Aluno->>VM: nova(320, 200)
    Aluno->>VM: carregue("nave.png")
    Note over VM: Value::Imagem; sem janela

    Aluno->>VM: apague + imagem(nave, 40, 100, 0)
    Aluno->>VM: mostre()
    VM->>Pipe: nasce expressa-grafico --ao-vivo
    VM->>Pipe: ABRA id=g 320 200
    VM->>Pipe: CARREGUE_FIGURA 1  (bytes RGBA, uma vez)
    VM->>Pipe: MOSTRE [imagem 1  40  100  0]
    Note over Visor: textura 1; apresenta
    Note over VM: mostre volta na hora

    loop cada quadro
        Aluno->>VM: apague + imagem(nave, x, y, angulo)
        Aluno->>VM: mostre()
        VM->>Pipe: MOSTRE [imagem 1  x  y  ang]
        Note over Visor: fundo, carimba handle 1
        Aluno->>VM: evento(0.05)
        VM->>Pipe: ESPERE id=g  50
        alt ninguém mexeu
            Visor->>Pipe: EVENTO nada
        else clique / tecla / X
            Visor->>Pipe: EVENTO clique ou tecla ou sair
        end
        Note over Aluno: angulo += 6, x += 2
    end

    Note over VM: fim do .lep ou :sair
    VM->>Pipe: fecha stdin
    Note over Visor: último quadro fica até o X
```

Volume típico: um PNG (ou RGBA) **uma vez**; depois dezenas de bytes por quadro + uma linha de `ESPERE`/`EVENTO`. 20–30 fps em 320×200–640×480 não satura o pipe. O teto visível é o `durma` / `evento(0.05)` e os ~30 fps do minifb.

## Quem implementa o host

| Host | `mostre` | `evento` |
|---|---|---|
| testes `NullGraf` | silêncio | erro (sem janela) |
| testes `CapturaGraf` | guarda a tela | fila de mapas |
| CLI `CliGrafHost` | pipe `ABRA`/`CARREGUE`/`MOSTRE` | `ESPERE` / lê `EVENTO` |
| Aula `ChannelGraf` | `ExecEvent::TelaGraf` (ops + `Arc` da figura, mesmo processo) | clique/tecla/fechar no `DrawingArea` → canal, como o `leia` |

No CLI o visor é o único lugar com minifb. A lib e a Aula não criam `Window`. Cartesiano continua no modo arquivo (`.cart` + `expressa-grafico <arquivo>`), sem clique.

Fim do programa: EOF no stdin do filho. A janela **permanece** com o último quadro até o aluno fechar o X (o mesmo hábito do visor mudo). Filho morto no próximo `mostre` / `evento` nasce de novo (`ABRA` + `CARREGUE` do que ainda está nas ops).

## Formato binário

Todos os inteiros little-endian. Depois do `u32` de tamanho, o primeiro byte é o tipo:

| byte | recado |
|---|---|
| 1 | `ABRA` — `u64 id`, `u32 w`, `u32 h`, fundo (`u16` len + UTF-8) |
| 2 | `CARREGUE_FIGURA` — `u32 handle`, `u32 w`, `u32 h`, `u32 n`, `n` bytes RGBA |
| 3 | `MOSTRE` — `u64 id`, `u32 nops`, ops |
| 4 | `ESPERE` — `u64 id`, `i32 timeout_ms` |
| 5 | `EVENTO` — `u64 id`, `u8 tipo`, `i32 x`, `i32 y`, `u8 botao`, tecla (`u16` len + UTF-8) |

Op: `u8` (`1` ponto, `2` linha, `3` circulo, `4` imagem, `5` texto, `6` retangulo) e campos `i32` / `f64` / cor UTF-8 / `u32` handle. Tipo de evento: `0` nada, `1` clique, `2` solte, `3` mova, `4` tecla, `5` sair. Botão: `0` nenhum, `1` esquerdo, `2` direito, `3` meio.

Teto de payload: 32 MiB (cabe um PNG no máximo da API, 2048² RGBA).
