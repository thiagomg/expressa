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

F5 roda, **F6 depura**, F7 mostra/oculta arquivos, F8 o painel do depurador, F9 (ou clique à esquerda do número da linha) ponto de parada, F10 próximo, F11 entrar, Ctrl+S salva.

A linha **argumentos** vai para `argumentos[1]…` no programa. `escreva_erro` aparece em vermelho. **Parar** encerra Rodar e Depurar. Clique na linha `erro: … em arquivo:linha` salta para o código. O título mostra `*` se houver alterações não salvas.

**Ctrl+F** busca (Esc fecha). Tab indenta `inicio`/`fim`/`{` `}` (Shift+Tab volta). O par de `inicio`/`fim` da linha do cursor fica destacado. Clique direito na árvore: novo arquivo/pasta, renomear, apagar. **Ctrl+clique** em `importe "matematica"` abre o módulo.

Fechar o editor encerra o `expressa-aula-server` que ele mesmo tiver iniciado. A janela abre maximizada; o código ocupa a maior parte da tela.

No depurador: a linha atual fica amarela; **Observados** guarda nomes para ver o valor a cada passo; **Variáveis** lista o que está no escopo; **Pilha** as chamadas. Continuar / Próximo / Entrar / Sair / Parar usam o mesmo passo a passo de `expressa debug`.

**projeto** é a pasta do aluno no servidor (`local` em casa). **Conectar** (ou Enter no nome) carrega a árvore de arquivos e pastas.

`leia("Seu nome:")` pede o texto na hora (o interpretador avisa com `<<<EXPRESSA-LEIA>>>`). Na linha de comando:

```bash
expressa --marcador-leia arquivo.lep
```

Home use: bind loopback, student name `local`. Lab: one server on the teacher PC; GTK clients talk RPC.
