# Expressa Aula (Freya)

Editor Freya da Expressa Aula. Corre **em paralelo** ao binário GTK
`expressa-aula` (`aula/ui`). Não substitui o GTK até o port estar completo.

```bash
LIBCLANG_PATH=/usr/lib/llvm-22/lib cargo run -p expressa-aula-freya
```

Se a GPU falhar:

```bash
FREYA_RENDERER=software LIBCLANG_PATH=/usr/lib/llvm-22/lib cargo run -p expressa-aula-freya
```

A janela abre maximizada, com menus **Arquivo / Executar / Depurar / Ver / Ajuda**,
barra de ferramentas e status `linha:coluna`. **Ver → Claro / Escuro** e
Ctrl+= / Ctrl+- / Ctrl+0 gravam as mesmas chaves do GTK em
`~/.config/expressa-aula/prefs` (`fonte`, `tema`, `pasta`, `arquivo`).
Ctrl+N / Ctrl+W abrem e fecham abas; Ctrl+S salva; Ctrl+G vai para a linha.
Rascunhos sujos são gravados a cada 5 s em `~/.cache/expressa-aula/rascunho-freya`.

Plano: `docs/tarefas/freya-tasks.md`.
