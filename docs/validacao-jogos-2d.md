# Validação do caminho 2D

Medições locais em 2026-11-14, com o executável release e janela real do CLI.
São resultados desta execução, não garantias de desempenho para outros computadores.

## Entrada assíncrona

`evento()` agora consulta um acumulador local. O visor envia mudanças
espontaneamente; uma thread recebe esses lotes sem depender dos callbacks da VM.
Somente transições não consumidas se acumulam; o conjunto de teclas mantidas é
substituído pelo mais recente. O pedido `ESPERE` e seus prazos foram removidos.

No release recompilado, com janela SDL2 real aberta e um segundo de aquecimento,
10 mil chamadas consecutivas de `evento()` levaram **70,32 ms** no total
(**7,03 µs por chamada**, incluindo execução na linguagem e medição externa).
Isso verifica a ausência da espera por refresh em cada consulta; não mede o
tempo entre pressionar uma tecla física e enxergar o resultado.

Os roteiros automáticos com essa implementação passaram:

| Alvo | Chamadas de eventos | Soma de dt | Cadência dos callbacks | Maior dt |
|---|---:|---:|---:|---:|
| 60 FPS | 194 | 3,242 s | 59,53/s | 17,2 ms |
| 30 FPS | 100 | 3,313 s | 29,88/s | 33,6 ms |

Os testes cobrem consultas sem pipe de escrita ou resposta do visor, persistência
de teclas mantidas, cem toques completos recebidos entre consultas, ordem e
modificadores, perda de foco, EOF com soltura das teclas e fechamento sem duplicar
`:sair`. O visor só publica mudanças e mantém a última cena após a saída da VM.
A coleta física no SDL ainda pode esperar o ciclo atual de VSync, sem bloquear
a simulação.

## Apresentação SDL2 com VSync — medição anterior à entrada assíncrona

O visor ao vivo agora apresenta uma textura completa pelo backbuffer acelerado
com VSync. Em OpenGL/GLES, a abertura só é aceita quando o driver confirma o
intervalo nativo de troca igual a 1; a emulação por temporizador do SDL não basta.
Nesta medição ainda havia pedidos de captura com prazo de 5 s na inicialização
e 400 ms depois. Esses pedidos e prazos foram removidos na entrada assíncrona.

Validação com o release recompilado e janela real, após essa mudança:

| Alvo | Chamadas de eventos | Soma de dt | Cadência dos callbacks | Maior dt |
|---|---:|---:|---:|---:|
| 60 FPS | 193 | 3,268 s | 58,75/s | 67,2 ms |
| 30 FPS | 100 | 3,294 s | 30,06/s | 52,9 ms |

Os dois roteiros terminaram com `validacao=verdadeiro`. Uma execução independente
pelo protocolo mediu 100 capturas: mediana de 16,63 ms, máximo de 72,68 ms e
12,40 ms para a captura após publicar mil quadros. Com VSync, a captura pode
esperar o ciclo atual do display. Essas medidas não são latência de teclado
físico até pixel nem uma inspeção visual de tearing no monitor do usuário.

Sete testes sem janela cobrem a tradução de teclas, repetição, soltura física,
modificadores simultâneos, perda de foco, mouse/letterbox, visibilidade e a
conversão ARGB com pitch. O renderizador reutiliza a textura e só atualiza seus
pixels quando chega um quadro novo.

As seções seguintes registram as medições históricas do apresentador minifb,
anteriores à migração. Validam o roteiro e a substituição de quadros, mas não
comprovam ausência de tearing, nem servem para comparar captura sem considerar
a espera introduzida pela sincronização do display.

## Executar

```sh
cargo build -p expressa --release --offline --locked
target/release/expressa exemplos/jogos/explorador.lep
target/release/expressa exemplos/jogos/explorador.lep --validar
target/release/expressa exemplos/jogos/explorador.lep --validar --30fps
```

O jogo tem mapa de 384×256 pixels, câmera de 240×144, painel de 16 pixels,
12 sprites e três cristais. Setas movem, Shift corre, R reinicia e Esc termina.
O último quadro permanece na janela ao terminar o programa; feche pelo X.
As imagens são construídas uma vez e reutilizadas, sem arquivos externos.

O roteiro automático usa a mesma simulação, os callbacks e o `dt` real do
modo interativo. Move até colidir com uma parede, solta os controles por
150 ms, segue até a abertura, atravessa e solta novamente. Verifica colisão,
imobilidade nas pausas e os dois quadros da animação. Falhas encerram o
programa com código 1. Uma pré-carga da cena antes do primeiro quadro evita
incluir seu carregamento inicial no movimento.

| Alvo | Chamadas de eventos | Duração medida por dt | Cadência dos callbacks | Maior dt |
|---|---:|---:|---:|---:|
| 60 FPS | 196 | 3,259 s | 59,83/s | 29,4 ms |
| 30 FPS | 101 | 3,330 s | 30,03/s | 39,3 ms |

A primeira chamada tem `dt = 0`; a cadência é `(chamadas - 1) / soma(dt)`.
Esses números medem atualizações do programa, não confirmações de apresentação
pelo compositor do sistema. O roteiro simula os controles e não injeta teclas
no teclado físico.

## Fila e captura

O CLI mantém um quadro completo pendente por janela e até oito comandos de
controle, retirados atomicamente para preservar dependências de imagens.
A Aula usa a mesma substituição por janela, com uma notificação coalescida;
pinta no máximo um lote por ciclo GTK e preserva o último lote ao terminar.

No teste de captura via protocolo, com 100 consultas sem esperar entrada:

| Medida | Resultado local |
|---|---:|
| Mediana da ida e volta de captura | 2,18 ms |
| Máximo observado | 6,62 ms |
| Captura após publicar mil quadros | 1,04 ms |

Essa latência é da consulta ao visor. Não é uma medida de teclado físico
até pixel na tela. Quadros substituídos não são rasterizados; abertura,
carregamento de imagens e entrada continuam ordenados.

## Cobertura automatizada

- Rajadas de 100 mil quadros: somente o último por janela fica pendente.
- Produção e consumo concorrentes: um quadro nunca precede suas imagens.
- Fila de controle limitada e leitor desbloqueado ao encerrar.
- EOF mantém o último quadro completo.
- Aula com consumidor atrasado: uma notificação e o último quadro de cada janela.
- Colisão, subpassos contra atravessamento de paredes e deslizamento lateral.
- Velocidade equivalente a 30/60 Hz e normalização de diagonais.
- Animação andando/parado, câmera, coleta e reinício.
- Exemplo completo nos executores AST e bytecode: soltura, toque rápido em R,
  fechamento e consumidor que retira apenas o quadro final.
- Imagens, formas e texto compostos em ordem, com transparência e PNG.

```sh
cargo test -p expressa --offline --locked
cargo test -p expressa-aula --offline --locked -- --test-threads=1
```

A suíte da Aula precisa de display gráfico. O jogo também expôs e permitiu
corrigir a composição antiga que pintava todas as imagens depois de formas
e texto, encobrindo o painel e os cristais.

## Conferência manual do teclado e foco

No modo interativo, segure uma seta por dois segundos e solte: personagem e
câmera devem parar no próximo quadro que consumir a soltura. Repita com duas
setas, Shift, setas opostas e troca de foco. Ao perder foco, as teclas devem
ser liberadas; recuperar foco não deve deixar o personagem andando sozinho.
Essa conferência cobre a entrega física de eventos pelo sistema e complementa
o roteiro e os testes de callbacks.
