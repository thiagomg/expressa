# Tela gráfica: janela e captura de eventos

A VM Expressa monta uma `TelaGrafica` e conversa com um host. O CLI usa
`expressa-grafico --ao-vivo`; a Aula usa janelas GTK. `mostre()` apresenta
sem esperar interação. `salve`, `pixel` e `como_imagem` rasterizam na VM.

## API

```expressa
tela_grafica = importe "tela_grafica"
g = tela_grafica::nova(160, 144, 4)
x = 80
g.tela_grafica::mostre()

enquanto verdadeiro {
    ev = g.tela_grafica::evento()
    se ev:sair ou ev:apertadas contem :escape { pare }
    se ev:teclas contem :esquerda { x -= 2 }
    se ev:teclas contem :direita { x += 2 }
    g.tela_grafica::apague()
    g.tela_grafica::retangulo(x, 100, x + 8, 108)
    g.tela_grafica::mostre()
    durma(0.02)
}
```

`evento()` captura o estado e consome todos os acontecimentos pendentes da
janela, sem esperar interação. Chame uma vez por quadro e reutilize o mapa.
Não recebe mais um tempo de espera: controle o ritmo do programa com `durma`.
Chame `mostre()` antes; no CLI, a primeira captura também pode abrir a janela.

| Campo | Conteúdo |
|---|---|
| `:teclas` | conjunto das teclas apertadas agora |
| `:apertadas` | conjunto das teclas apertadas desde a captura anterior, sem repetição automática |
| `:soltas` | conjunto das teclas soltas desde a captura anterior |
| `:eventos` | lista dos acontecimentos, em ordem |
| `:sair` | verdadeiro se houve pedido para fechar |

| Ação com direita | `:teclas` | `:apertadas` | `:soltas` |
|---|---|---|---|
| Aperta | direita | direita | — |
| Continua segurando | direita | — | — |
| Solta | — | — | direita |
| Continua solta | — | — | — |

Um toque completo entre duas capturas aparece em `:apertadas` e `:soltas`,
mas não em `:teclas`; `:eventos` preserva sua ordem. A repetição automática do
sistema é ignorada. Ao perder foco ou fechar, o estado das teclas é limpo e
as teclas que estavam apertadas aparecem em `:soltas`. Cada janela tem estado
e acontecimentos independentes. Modificar um mapa retornado não altera o host.

### Teclas e modificadores

Todos os nomes são textos em minúsculas: `:direita == "direita"`.

| Grupo | Nomes |
|---|---|
| Letras | `:a` a `:z` |
| Números | `:num_0` a `:num_9` |
| Funções | `:f1` a `:f15` |
| Direções | `:esquerda`, `:direita`, `:cima`, `:baixo` |
| Ações | `:espaco`, `:enter`, `:escape`, `:tab`, `:backspace` |
| Navegação | `:inicio`, `:fim`, `:pagina_acima`, `:pagina_abaixo`, `:insert`, `:delete` |
| Pontuação | `:apostrofo`, `:crase`, `:barra_invertida`, `:virgula`, `:igual`, `:colchete_esquerdo`, `:colchete_direito`, `:menos`, `:ponto`, `:ponto_e_virgula`, `:barra` |
| Outras | `:menu`, `:pause`, `:caps_lock`, `:num_lock`, `:scroll_lock` |
| Teclado numérico | `:numerico_0` a `:numerico_9`, `:numerico_ponto`, `:numerico_barra`, `:numerico_asterisco`, `:numerico_menos`, `:numerico_mais`; Enter usa `:enter` |

| Modificador | Tecla no teclado |
|---|---|
| `:shift` | Shift de qualquer lado |
| `:control` | Control/Ctrl de qualquer lado |
| `:alt` | Alt de qualquer lado; Option no macOS; AltGr na Aula |
| `:super` | Windows/Super; Command no macOS |

Os modificadores participam dos mesmos conjuntos. Soltar um Shift não remove
`:shift` enquanto o outro estiver apertado. A disponibilidade de teclas e
atalhos depende do sistema, teclado e backend. Caps Lock, Num Lock e Scroll
Lock representam pressionamento físico, não uma consulta da trava ligada.
Letras não mudam com Shift/Caps Lock. Esta API é para controles e atalhos,
não para digitação de texto; o CLI usa as posições de teclas do minifb e a
Aula usa o mapa de teclado do sistema. A Aula também reconhece letras locais.

### Acontecimentos em ordem

Cada item de `:eventos` contém `:tipo`, `:tecla`, `:x`, `:y`, `:botao`,
`:modificadores` (conjunto) e `:repeticao` (falso, pois repetições são ignoradas).
Tipos: `:tecla`, `:tecla_solta`, `:clique`, `:solte`, `:desfoco` e `:sair`.
`solte` é soltura de mouse; `tecla_solta` é soltura de teclado. `desfoco` e
`sair` também sinalizam a liberação de todas as teclas pelo campo `:soltas`.

```expressa
para acontecimento em ev:eventos {
    se acontecimento:tipo == :tecla e acontecimento:tecla == :s {
        se acontecimento:modificadores contem :control {
            // Control estava apertado quando S foi apertado,
            // mesmo que Control já tenha sido solto antes da captura.
            salve_jogo()
        }
    }
}
```

Os eventos de mouse têm `:x` e `:y` em pixels lógicos, origem no canto
superior esquerdo, y para baixo. `:botao` é `:esquerdo`, `:direito` ou `:meio`
(`:nenhum` nos demais eventos). As faixas de letterbox não geram cliques.

Veja [o exemplo de movimento](../exemplos/tela_grafica/movimento.lep) e a
[referência completa de nativas](nativas/tela_grafica.lep), também usada pela
ajuda da linguagem e pelos editores.

## Hosts e transporte

Um filho por tela no CLI; na Aula, eventos de cada janela viajam com seu id
por um canal para a thread da VM. Os hosts usam o mesmo acumulador de estado.
O CLI captura as transições de teclado por callback do minifb, preservando
pressionamentos e solturas que chegam na mesma atualização da janela.

Mensagens com prefixo de tamanho (`u32` little-endian + payload), limite
32 MiB. O pipe é do processo gráfico e não interfere em `leia`.

| Byte | Mensagem |
|---|---|
| 1 | `ABRA`: `u64 id`, `u32 largura`, `u32 altura`, `u32 tamanho_pixel`, texto fundo |
| 2 | `CARREGUE_FIGURA`: `u32 handle`, `u32 largura`, `u32 altura`, `u32 n`, n bytes RGBA |
| 3 | `MOSTRE`: `u64 id`, `u32 nops`, operações |
| 4 | `ESPERE`: `u64 id` — pede uma captura, sem timeout |
| 5 | `EVENTO`: `u64 id`, conjuntos teclas/apertadas/soltas, `u8 sair`, `u32 n`, n acontecimentos |

Texto: `u16 tamanho` + UTF-8. Conjunto: `u32 quantidade` + textos.
Acontecimento: texto tipo, `i32 x`, `i32 y`, texto botão, texto tecla,
texto identidade física, conjunto modificadores, `u8 repeticao`.
A identidade física é interna; o mapa da linguagem não a expõe.

`MOSTRE` não espera ACK. `ESPERE` recebe exatamente uma resposta `EVENTO`
com todos os acontecimentos pendentes e o estado atual. O nome interno
`ESPERE` não indica espera por entrada do usuário. A primeira aparição de
cada imagem envia seus pixels; os quadros seguintes reutilizam o handle.
Operações: ponto, linha, círculo, imagem, texto e retângulo.

Fim do programa: EOF no stdin do filho. A janela mantém o último quadro até
fechar. Parar na Aula interrompe a execução; fechar uma janela limpa seu
estado. `NullGraf` rejeita capturas sem janela; `CapturaGraf` permite injetar
transições em testes.
