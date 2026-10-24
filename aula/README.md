# Expressa Aula

IDE integrada para ser usada por alunos

| Crate | Função |
|-------|--------|
| `expressa` (raiz do repositório) | Linguagem: analisador léxico, analisador sintático, executor |
| `expressa-aula-proto` | protobuf do gRPC |
| `expressa-aula-server` | Repositório de arquivos: diretórios dos estudantes e local |
| `expressa-aula` (`aula/ui`) | editor de texto: lista, Executar (F5) / Depurar (F6) no próprio processo |

## Layout no disco

```text
--root /var/expressa-aula
  local/           # loopback / casa
  ana/
    bhaskara.lep
  bruno/
```

## Execução dos programas

O editor executa e depura os programas; o servidor só guarda arquivos. Em Executar/Depurar o editor copia o projeto inteiro para `~/.cache/expressa-aula/{projeto}/` (`$XDG_CACHE_HOME` se estiver definido), grava o texto do editor por cima do arquivo aberto e roda ali. Quando o programa termina, arquivos novos ou alterados (o fonte que rodou, `salve_arquivo`, `salve_csv`, …) voltam para o servidor.

A execução não lê nem grava fora da pasta do projeto (`..`, caminhos absolutos). Não há limite de tempo: jogos e outros laços infinitos (`exemplos/jogos/conway_jogo_da_vida.lep`) continuam até **Parar**. O painel de saída guarda as últimas 5000 linhas; depois de `cls()`/`casa()` funciona como um terminal (cada linha substitui o quadro anterior).

## Subir o servidor

```bash
# servidor (se já não estiver no ar, o editor tenta iniciá-lo)
cargo run -p expressa-aula-server -- --root ./aula-data --bind 127.0.0.1:50051

# editor
cargo run -p expressa-aula
```

Na primeira abertura (e de novo depois de atualizar o pacote) o editor copia os exemplos empacotados para `Documentos/Expressa Aula/exemplos` (ou `~/Documentos/…` no Linux). Arquivos novos entram na pasta; o que o aluno já tiver ali não é substituído.

Os comandos ficam no menu do topo (**Arquivo**, **Executar**, **Depurar**, **Ver**, **Ajuda**), cada um com o atalho à direita. **Arquivo → Sair** (Ctrl+Q) fecha o editor (com pergunta se houver alterações). F5 executa a aba atual, **F6 depura**, F7 mostra/oculta arquivos, F8 o painel do depurador, F9 (ou clique no número da linha) ponto de parada, F10 próximo, F11 entrar, Shift+F11 sair, Ctrl+S salva. O caminho do projeto aparece à direita da barra de baixo. `mostre()` do módulo `cartesiano` abre o painel **Gráficos** (Destacar vira janela irmã e fica assim até Acoplar ou fechar). O desenho ocupa o retângulo que o painel ou a janela lhe dão, como o `expressa-grafico`.

A linha **argumentos** vai para `argumentos[1]…` no programa. `escreva_erro` aparece em vermelho. **Parar** encerra Executar e Depurar. Clique na linha `erro: … em arquivo:linha` abre o arquivo na linha. Executar usa o texto de todas as abas abertas, salvas ou não.

**Abas**: cada arquivo abre na sua aba, com desfazer (Ctrl+Z / Ctrl+Shift+Z) e pontos de parada próprios; `●` marca alterações não salvas. Ctrl+N abre uma aba nova, Ctrl+W fecha. Fechar uma aba, trocar de projeto ou fechar a janela com alterações pergunta se quer salvar.

**Edição**: Tab indenta como no Emacs (com linhas selecionadas, desloca o bloco; Shift+Tab volta), Ctrl+Alt+\\ reindenta a seleção, Enter já indenta a linha nova e `}`/`fim` voltam sozinhos. Ctrl+/ comenta ou descomenta, Ctrl+D duplica a linha, Alt+↑/↓ move. O par `inicio`/`fim` ou `{`/`}` do cursor fica destacado. **Ctrl+F** busca (Esc fecha). Clique direito na árvore (arquivo, pasta ou área vazia): novo arquivo/pasta (na pasta clicada), renomear, apagar. Delete também apaga o item selecionado.

**Ajuda da linguagem**: **Ajuda → Funções nativas** (ou **F1** fora de um nome) abre uma janela com todas as funções nativas (núcleo, `tela`, `mat`, `matriz`, `arquivo`, `cartesiano`) e a ficha de cada uma (`ajuda("matriz::zeros")`). Ao digitar aparecem as funções nativas, as do arquivo e as dos `importe` (escolher `pinte` acrescenta `importe "tela"`); dentro de `importe "` aparecem os módulos e os arquivos. Dentro dos parênteses, a assinatura mostra o argumento atual. Passar o mouse ou **F1** no nome mostra a ficha. **Esc** fecha o autocomplete; ao continuar digitando a lista volta. **Ctrl+clique** ou **F12** vai para a definição (inclusive em outro arquivo e no próprio `importe`). Erros de sintaxe aparecem sublinhados enquanto digita, com a mensagem na barra de baixo.

**Sala de aula**: Ctrl+= / Ctrl+- (ou A+ / A−) aumentam e diminuem a letra, Ctrl+0 volta. **Ver → Claro** ou **Ver → Escuro** pinta a janela inteira (menus, painéis, editor e saída). A escolha fica em `~/.config/expressa-aula/prefs`.

Fechar o editor fecha as janelas irmãs (gráficos destacados e Ajuda) e encerra o `expressa-aula-server` que ele mesmo tiver iniciado. A janela abre maximizada; o código ocupa a maior parte da tela. Em janela estreita os painéis encolhem.

No depurador: a linha atual fica amarela, no arquivo certo (abre o módulo importado se for preciso); um erro durante a depuração para na linha (em vermelho) com as variáveis visíveis, e Continuar encerra. **Observados** guarda nomes para ver o valor a cada passo; **Variáveis** lista o que está no escopo; **Pilha** as chamadas.

**Arquivo → Pasta…** escolhe a pasta do projeto neste computador; **Arquivo → Servidor…** conecta a um servidor (`aula-data/aluno`). O caminho (ou `url — aula-data/aluno`) fica à direita da barra de status.

`leia("Seu nome:")` espera a resposta na própria saída: digite e aperte Enter, como num terminal (o interpretador avisa com `<<<EXPRESSA-LEIA>>>`). Na linha de comando:

```bash
expressa --marcador-leia arquivo.lep
```

Em casa: o servidor escuta no loopback, aluno `local`. Na sala: um servidor no computador do professor; os clientes GTK falam por RPC.
