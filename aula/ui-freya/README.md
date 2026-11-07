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
Ctrl+F busca no arquivo; Tab / Shift+Tab recuam; Ctrl+/ comenta; Ctrl+Alt+\\
reindenta. Palavras Expressa (função, início, senão) ganham destaque via
`aula/tree-sitter-expressa`. Erros de sintaxe aparecem na barra de status.
F7 mostra a árvore do projeto; **Arquivo → Abrir projeto…** escolhe a pasta.
Na primeira abertura copia exemplos em `Documentos/Expressa Aula/exemplos`
sem sobrescrever arquivos já editados.
F5 roda o texto da aba (mesmo sem salvar) no interpretador do processo;
a saída aparece no console. `leia` pede a resposta ali mesmo; **Parar**
interrompe um laço infinito. A cópia de execução fica em
`~/.local/share/expressa-aula/exec-freya` (ou o cache do sistema).
F1 mostra a ficha da nativa sob o cursor; fora de um nome abre o catálogo.
Ctrl+Espaço sugere nomes; Ajuda → Tutoriais abre `expressa-tutorial`.
F6 inicia o depurador (pausa na primeira linha); F9 marca ponto; F10 / F11 /
Shift+F11 avançam. F5 com o programa pausado continua. F8 mostra ou esconde
o painel (Observados, Variáveis, Pilha). A fonte que roda é a cópia em
`exec-freya`: editar a aba no meio da execução não muda o que já está rodando.
**Ver → REPL** abre um painel: as variáveis ficam entre linhas; Parar interrompe
um laço; Reiniciar esquece a sessão. `saia()` encerra até o próximo Reiniciar.
`mostre()` de `cartesiano` e `tela_grafica` abre o painel Gráficos acima do
console. Destacar solta a janela; clicar na tela envia `evento()`. Fechar X
não recria a tela até o próximo F5.
Rascunhos sujos são gravados a cada 5 s em `~/.cache/expressa-aula/rascunho-freya`.

Plano: `docs/tarefas/freya-tasks.md`.
