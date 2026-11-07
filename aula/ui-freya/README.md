# Expressa Aula (Freya)

Editor Freya da Expressa Aula. Corre **em paralelo** ao binário GTK
`expressa-aula` (`aula/ui`). O GTK permanece; este crate não o substitui.
Correções: `freya-tasks/`.

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
`aula/tree-sitter-expressa`. Erro de sintaxe estável marca a linha no gutter;
a barra de status fica para F5 e recados da IDE.
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
Na próxima abertura, **Recuperar** devolve esses rascunhos; **Descartar** apaga.
Ctrl+S grava com proteção contra conflito (CAS). Se o arquivo mudou no disco
ou no servidor, o diálogo oferece Comparar, Recarregar, Salvar como e Substituir.
**Arquivo → Servidor…** conecta na Turma (em casa: `http://127.0.0.1:50051`,
aluno `local`). Se o servidor não estiver no ar, o editor tenta subir
`expressa-aula-server` no loopback com raiz em
`~/.local/share/expressa-aula/aula-data`. A árvore remota usa um cache em
`…/expressa-aula/remoto/{aluno}/{projeto}` para o F5 continuar local.

Plano: `docs/tarefas/freya-tasks.md`.

## Pacote

O `.deb` (`./empacotar-deb.sh`) instala `/usr/bin/expressa-aula-freya` **ao
lado** de `expressa-aula` (GTK). O binário Freya não liga GtkSourceView; o
pacote ainda depende de GTK porque o editor da sala continua no mesmo `.deb`.
O `build-inside.sh` imprime o tamanho dos dois após o `strip` para comparar.

Windows: `empacotar-freya.ps1` gera `dist/expressa-aula-freya/` e o zip **sem**
DLLs do GTK. Compile no Windows (MSVC, rust-skia); não há cross a partir do
Linux. O instalador Inno do GTK (`criar-instalador.ps1`) não foi substituído.

Tamanhos após `strip` neste computador (2026-11-07, Linux x86_64):

| Binário | Após strip |
|---|---|
| `expressa-aula` (GTK) | 8,4 MiB |
| `expressa-aula-freya` (Skia estático) | 37 MiB |

Meça de novo com `strip` numa cópia de `target/release/expressa-aula-freya`
depois de `cargo build --release`. O `.deb` já faz `strip --strip-unneeded`
nos dois (`packaging/deb/build-inside.sh`).

## Windows (máquina nativa)

Não há cross a partir do Linux. No Windows (MSVC, VS 2022 C++):

```text
powershell -NoProfile -ExecutionPolicy Bypass -File .\empacotar-freya.ps1
```

O zip `dist/expressa-aula-freya-windows.zip` não leva DLLs GTK. O instalador
Inno da Aula GTK (`criar-instalador.ps1`) **permanece**.

Walkthrough (IME winit, não IBus/GTK):

1. `função`, `não`, `início`, `ç` — sem quadrados (tarefa 21: Cascadia/Consolas).
2. Arquivo → Abrir projeto… e um exemplo em Documentos.
3. F5, `leia` no console, Parar.
4. Ver → Escuro / Claro.
5. GPU má: `FREYA_RENDERER=software` (o `LEIA-ME.txt` do zip explica).

O binário Freya **não** passa a chamar-se `expressa-aula`. `aula/ui` não se arquiva.
