# Expressa Aula

Classroom editor services for Expressa. **Texted is a separate project and is not used here.**

| Crate | Role |
|-------|------|
| `expressa` (repo root) | Language: lexer, parser, runner |
| `expressa-aula-proto` | gRPC: `Turma` (files) |
| `expressa-aula-server` | File repository: student folders only, runs nothing |
| `expressa-aula` (`aula/ui`) | GTK editor: lista, código, Rodar (F5) / Depurar (F6) in-process |

## Why this repo, not Texted

The language still changes often (`raiz`, debugger, sandbox). Aula must call `expressa::runtime` in the same commit. A second Git repo would lag. Texted stays a markdown product.

## Layout on disk

```text
--root /var/expressa-aula
  local/           # loopback / home
  ana/
    bhaskara.lep
  bruno/
```

## Running programs

The editor runs and debugs programs itself; the server only stores files. On Rodar/Depurar the editor copies the whole project into `~/.cache/expressa-aula/{projeto}/` (`$XDG_CACHE_HOME` if set), writes the editor text over the open file, and runs there. When the program ends, files that are new or changed (the source that ran, `salve_arquivo`, `salve_csv`, …) are sent back to the server.

Runs cannot read or write outside the project folder (`..`, absolute paths). There is no time limit, so games and other endless loops (`exemplos/jogos/conway_jogo_da_vida.lep`) keep running until **Parar**. The output pane keeps the last 5000 lines; after `cls()`/`casa()` it behaves like a terminal (each line overwrites the previous frame).

## Run the server

```bash
# servidor (se já não estiver no ar, o editor tenta iniciá-lo)
cargo run -p expressa-aula-server -- --root ./aula-data --bind 127.0.0.1:50051

# editor
cargo run -p expressa-aula
```

F5 roda a aba atual, **F6 depura**, F7 mostra/oculta arquivos, F8 o painel do depurador, F9 (ou clique no número da linha) ponto de parada, F10 próximo, F11 entrar, Shift+F11 sair, Ctrl+S salva.

A linha **argumentos** vai para `argumentos[1]…` no programa. `escreva_erro` aparece em vermelho. **Parar** encerra Rodar e Depurar. Clique na linha `erro: … em arquivo:linha` abre o arquivo na linha. Rodar usa o texto de todas as abas abertas, salvas ou não.

**Abas**: cada arquivo abre na sua aba, com desfazer (Ctrl+Z / Ctrl+Shift+Z) e pontos de parada próprios; `●` marca alterações não salvas. Ctrl+N abre uma aba nova, Ctrl+W fecha. Fechar uma aba, trocar de projeto ou fechar a janela com alterações pergunta se quer salvar.

**Edição**: Tab indenta como no Emacs (com linhas selecionadas, desloca o bloco; Shift+Tab volta), Ctrl+Alt+\\ reindenta a seleção, Enter já indenta a linha nova e `}`/`fim` voltam sozinhos. Ctrl+/ comenta ou descomenta, Ctrl+D duplica a linha, Alt+↑/↓ move. O par `inicio`/`fim` ou `{`/`}` do cursor fica destacado. **Ctrl+F** busca (Esc fecha). Clique direito na árvore: novo arquivo/pasta, renomear, apagar.

**Ajuda da linguagem**: o botão **Ajuda** (ou **F1** fora de um nome) abre uma janela com todas as funções nativas (núcleo, `tela`, `mat`, `matriz`, `arquivo`) e a ficha de cada uma (`ajuda("matriz::zeros")`). Ao digitar aparecem as funções nativas, as do arquivo e as dos `importe` (escolher `pinte` acrescenta `importe "tela"`); dentro de `importe "` aparecem os módulos e os arquivos. Dentro dos parênteses, a assinatura mostra o argumento atual. Passar o mouse ou **F1** no nome mostra a ficha. **Esc** fecha o autocomplete; ao continuar digitando a lista volta. **Ctrl+clique** ou **F12** vai para a definição (inclusive em outro arquivo e no próprio `importe`). Erros de sintaxe aparecem sublinhados enquanto digita, com a mensagem na barra de baixo.

**Sala de aula**: Ctrl+= / Ctrl+- (ou A+ / A−) aumentam e diminuem a letra, Ctrl+0 volta; **Escuro** troca o tema. As duas escolhas ficam salvas em `~/.config/expressa-aula/prefs`.

Fechar o editor encerra o `expressa-aula-server` que ele mesmo tiver iniciado. A janela abre maximizada; o código ocupa a maior parte da tela.

No depurador: a linha atual fica amarela, no arquivo certo (abre o módulo importado se for preciso); um erro durante a depuração para na linha (em vermelho) com as variáveis visíveis, e Continuar encerra. **Observados** guarda nomes para ver o valor a cada passo; **Variáveis** lista o que está no escopo; **Pilha** as chamadas.

**projeto** é a pasta do aluno no servidor (`local` em casa). **Conectar** (ou Enter no nome) carrega a árvore de arquivos e pastas.

`leia("Seu nome:")` espera a resposta na própria saída: digite e aperte Enter, como num terminal (o interpretador avisa com `<<<EXPRESSA-LEIA>>>`). Na linha de comando:

```bash
expressa --marcador-leia arquivo.lep
```

Home use: bind loopback, student name `local`. Lab: one server on the teacher PC; GTK clients talk RPC.
