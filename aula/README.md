# Expressa Aula

Classroom editor services for Expressa. **Texted is a separate project and is not used here.**

| Crate | Role |
|-------|------|
| `expressa` (repo root) | Language: lexer, parser, runner |
| `expressa-aula-proto` | gRPC: `Turma` (files) + `Runner` (execute) |
| `expressa-aula-server` | Sandboxed backend: student folders + `run_to_string_with` |
| `expressa-aula` (`aula/ui`) | GTK editor: lista, código, Rodar (F5) |

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

Runs cannot read or write outside the student folder (`..`, absolute paths). A time limit stops `repita` that never ends.

## Run the server

```bash
# servidor (se já não estiver no ar, o editor tenta iniciá-lo)
cargo run -p expressa-aula-server -- --root ./aula-data --bind 127.0.0.1:50051

# editor
cargo run -p expressa-aula
```

F5 roda, **F6 depura**, F7 mostra/oculta arquivos, F8 o painel do depurador, F9 (ou clique à esquerda do número da linha) ponto de parada, F10 próximo, F11 entrar, Ctrl+S salva. Fechar o editor encerra o `expressa-aula-server` que ele mesmo tiver iniciado. A janela abre maximizada; o código ocupa a maior parte da tela.

No depurador: a linha atual fica amarela; **Observados** guarda nomes para ver o valor a cada passo; **Variáveis** lista o que está no escopo; **Pilha** as chamadas. Continuar / Próximo / Entrar / Sair / Parar usam o mesmo passo a passo de `expressa debug`.

**projeto** é a pasta do aluno no servidor (`local` em casa). **Conectar** (ou Enter no nome) carrega a árvore de arquivos e pastas.

`leia("Seu nome:")` pede o texto na hora (o interpretador avisa com `<<<EXPRESSA-LEIA>>>`). Na linha de comando:

```bash
expressa --marcador-leia arquivo.lep
```

Home use: bind loopback, student name `local`. Lab: one server on the teacher PC; GTK clients talk RPC.
