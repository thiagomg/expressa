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

Plano: `docs/tarefas/freya-tasks.md`.
