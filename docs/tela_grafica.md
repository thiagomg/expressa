# Tela gráfica: janela e captura de eventos

A VM Expressa monta uma `TelaGrafica` e conversa com um host. O CLI usa
`expressa-grafico --ao-vivo`; a Aula usa janelas GTK. `mostre()` apresenta
sem esperar interação. `salve`, `pixel` e `como_imagem` rasterizam na VM.

## Laço gráfico

Para animações e jogos 2D, o laço controla entrada, atualização, desenho e espera:

```expressa
tela_grafica = importe "tela_grafica"
g = tela_grafica::nova(160, 144, 4)
l = g.tela_grafica::laço_grafico()
personagem = mapa([:x -> 20])

eventos = função(ev, dt) {
    se ev:teclas contem :direita { personagem:x += 60 * dt }
    se ev:apertadas contem :escape { l.tela_grafica::pare() }
}

desenhe = função(tela) {
    tela.tela_grafica::retangulo(personagem:x, 80, personagem:x + 8, 88)
}

config = mapa([
    :fps -> 30,
    :eventos -> eventos,
    :desenho -> desenhe,
])
l.tela_grafica::execute(config)
```

`laço_grafico()` (também `laco_grafico()`) cria o recurso sem abrir janela.
`execute(config)` valida as três opções obrigatórias e inicia a execução,
bloqueando até o encerramento. Opções desconhecidas dão erro. O `execute` deste
módulo é independente de `processo::execute`.

A cada quadro, captura a entrada e chama `eventos(ev, dt)`, mesmo sem novos
acontecimentos. `ev` tem exatamente os campos descritos abaixo para `evento()`.
`dt` mede o tempo real entre chamadas, em segundos, com relógio monotônico;
na primeira chamada é zero. Use velocidade em pixels por segundo multiplicada
por `dt`. Depois o laço apaga o desenho, chama `desenhe(g)` e apresenta o quadro.
Não é necessário chamar `evento()`, `apague()`, `mostre()` ou `durma()` nos callbacks.
Os mapas compartilhados, como `personagem`, permitem que ambos os callbacks
acessem o estado do programa sem alterar variáveis externas à função.

O tempo gasto na captura, nos callbacks e na apresentação é descontado da
espera até o próximo quadro. Um quadro lento não cria uma fila de atualizações
para recuperar o atraso: o próximo começa sem espera, com o tempo real em `dt`.
`:fps` é uma frequência desejada; o programa e o visor podem limitar o resultado.
A configuração é capturada ao iniciar, não é relida a cada quadro.

Fechar a janela entrega uma última chamada com `ev:sair` verdadeiro e termina
sem desenhar outro quadro. `l.tela_grafica::pare()` solicita terminar depois do
callback atual, sem apresentar outro quadro e sem fechar a janela. Também são
aceitos `pare_laco()` e `pare_laço()`. Erros dos callbacks são propagados; o
botão Parar da Aula e os limites de tempo continuam funcionando durante a espera.

`execute()` devolve o laço. É possível executá-lo novamente após terminar,
inclusive após tratar uma falha com `se_falhar`. Um laço não pode ser clonado;
somente um laço pode estar ativo por tela. `g.evento()` continua disponível
para programas que controlam seu próprio ciclo.

## Captura manual de eventos

```expressa
importe "sistema"
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
não para digitação de texto; o CLI usa os nomes de teclas do SDL e acompanha cada tecla
física para reconhecer sua soltura. A Aula usa o mapa de teclado do sistema
e também reconhece letras locais.

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
O visor ao vivo do CLI usa SDL2: captura as transições de teclado em ordem
e apresenta o quadro completo por uma textura ARGB com VSync. A apresentação
usa o backbuffer do renderizador acelerado, com escala por vizinho mais próximo.
Os pixels da textura são reutilizados enquanto o desenho não muda. O backend
de vídeo é escolhido pelo SDL (incluindo Wayland nativo quando disponível);
o caminho ao vivo não força mais X11 nem usa `XPutImage`.

VSync é solicitado na criação, ativado explicitamente e confirmado no renderer.
Em OpenGL/GLES, o intervalo nativo de troca também é consultado: o temporizador
de emulação do SDL não é aceito como confirmação de VSync.
Se estiver indisponível, o visor informa o erro; não troca silenciosamente para
uma apresentação sem sincronização. O relógio de apresentação segue o display,
sem um segundo limitador fixo de 60 FPS. `:fps` continua sendo a frequência
desejada dos callbacks, limitada também pelo visor e pelo hardware.

O visor publica as mudanças de entrada espontaneamente, antes de apresentar.
Uma thread no processo principal recebe os lotes e acumula as transições por
janela. `evento()` consulta esse estado local imediatamente, sem enviar pedidos,
ler o pipe ou esperar pelo VSync. Teclas mantidas usam o estado mais recente;
pressionamentos, solturas e acontecimentos permanecem até a próxima consulta.
A consulta pode retornar vazia enquanto a janela ainda está inicializando.

A coleta no SDL ainda ocorre na thread de apresentação: uma mudança física pode
esperar o ciclo atual de VSync para ser coletada. Isso não bloqueia a simulação.
A pausa de 2 ms fica apenas para janelas ocultas ou sem quadro.
Teclas, modificadores, soltura, foco, mouse e letterbox preservam a mesma API.
A Aula continua usando a apresentação do GTK. Os visualizadores estáticos de
arquivos `.cart`/PNG continuam em minifb. No Linux, esse visualizador usa
X11 (via XWayland em sessões Wayland) para exibir a moldura e o botão de fechar
também no GNOME. Ele exige um servidor X11/XWayland disponível e pode ser fechado
pelo botão da janela ou pela tecla Esc.

### Quadros pendentes

Há somente **um quadro completo pendente por janela**, além do quadro em
exibição e dos buffers limitados do transporte. Um novo `mostre()` substitui
o quadro ainda não apresentado. A substituição acontece antes da conversão
e rasterização; não é necessário desenhar os quadros descartados.

No CLI, a thread que lê o pipe guarda o último `MOSTRE` e mantém uma fila
limitada a oito comandos de controle. Abertura, carregamento de imagens e
pedidos de entrada continuam ordenados. O visor retira os controles e o
quadro sob o mesmo bloqueio, garantindo que suas imagens já estejam
carregadas. Fechar o pipe preserva o último quadro pendente.

Na Aula, a VM publica o quadro em uma caixa compartilhada. Uma única
notificação avisa o GTK de que há quadros pendentes. O GTK apresenta no máximo
um lote por ciclo de sua interface, inclusive quando o produtor é mais rápido.
O último lote é apresentado antes de concluir a execução. Console, depuração
e término continuam em seu canal ordenado; eventos de entrada não são
substituídos junto com os desenhos.

### Composição

Imagens, formas e texto respeitam a ordem dos comandos. Um texto ou retângulo
após uma imagem aparece sobre ela. O mesmo compositor é usado no CLI, na
Aula, em `como_imagem()` e na exportação PNG.

### Validação com um jogo

[Explorador](../exemplos/jogos/explorador.lep) inclui mapa sólido, câmera,
12 sprites de personagem, animação ao andar, colisão com deslizamento nas
paredes, movimento diagonal normalizado e coleta de cristais. O mapa e os
sprites viram imagens uma vez, reutilizadas nos quadros seguintes.

```sh
target/release/expressa exemplos/jogos/explorador.lep
target/release/expressa exemplos/jogos/explorador.lep --validar
target/release/expressa exemplos/jogos/explorador.lep --validar --30fps
```

Setas movem, Shift corre, R reinicia e Esc termina. `--validar` usa o relógio
real e o mesmo laço/renderizador, conduzindo automaticamente o personagem
até uma parede, soltando os controles, passando por uma abertura e parando.
O resultado no terminal verifica colisão, imobilidade ao soltar e os dois
quadros de caminhada; também informa duração, quadros e maior `dt` observado.
Esse roteiro simula os controles: não substitui a verificação manual do
teclado físico e do foco em cada sistema.

As medições e o procedimento estão em [Validação do caminho 2D](validacao-jogos-2d.md).

Os testes automatizados também cobrem 30/60 atualizações por segundo,
colisão em quadros lentos, câmera, coleta, reinício por toque rápido,
fechamento da janela e um consumidor que só retira o último quadro no fim.

Mensagens com prefixo de tamanho (`u32` little-endian + payload), limite
32 MiB. O pipe é do processo gráfico e não interfere em `leia`.

| Byte | Mensagem |
|---|---|
| 1 | `ABRA`: `u64 id`, `u32 largura`, `u32 altura`, `u32 tamanho_pixel`, texto fundo |
| 2 | `CARREGUE_FIGURA`: `u32 handle`, `u32 largura`, `u32 altura`, `u32 n`, n bytes RGBA |
| 3 | `MOSTRE`: `u64 id`, `u32 nops`, operações |
| 5 | `EVENTO`: `u64 id`, conjuntos teclas/apertadas/soltas, `u8 sair`, `u32 n`, n acontecimentos |

Texto: `u16 tamanho` + UTF-8. Conjunto: `u32 quantidade` + textos.
Acontecimento: texto tipo, `i32 x`, `i32 y`, texto botão, texto tecla,
texto identidade física, conjunto modificadores, `u8 repeticao`.
A identidade física é interna; o mapa da linguagem não a expõe.

`MOSTRE` não espera ACK. `EVENTO` é enviado espontaneamente quando há mudanças,
com as transições desde o último envio e o estado atual. O processo principal
une as transições recebidas até `evento()` consumi-las, mantendo o último estado
de teclas. O antigo pedido `ESPERE` (byte 4) foi removido.
A primeira aparição de cada imagem envia seus pixels; os quadros seguintes
reutilizam o handle.
Operações: ponto, linha, círculo, imagem, texto e retângulo.

Fim do programa: EOF no stdin do filho. A janela mantém o último quadro até
fechar. Parar na Aula interrompe a execução; fechar uma janela limpa seu
estado. `NullGraf` rejeita capturas sem janela; `CapturaGraf` permite injetar
transições em testes.
