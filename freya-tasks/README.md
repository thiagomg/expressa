# Tarefas do editor Freya

Lista de correções de `expressa-aula-freya` (`aula/ui-freya`), uma por arquivo.
O binário GTK `expressa-aula` (`aula/ui`) **permanece**: estes itens não
arquivam nem substituem a Aula GTK. Os dois editores seguem no mesmo
pacote. Trabalho só em `aula/ui-freya`, salvo extrair lógica pura já
compartilhável (indent, console CSI, lang) sem tirar o GTK.

Ordem sugerida: P0 (01–06), depois layout (07), depurador e gráficos
(08–11), atalhos/servidor/diálogos/ajuda (12–15), polimento (16–25).

## P0 — uso diário

| Arquivo | O quê |
|---|---|
| [01-editar-com-historico.md](01-editar-com-historico.md) | Editar sem `set()`: undo e dirty |
| [02-indent-depois-da-tecla.md](02-indent-depois-da-tecla.md) | Não interceptar `m` e `}` |
| [03-console-csi-quadro.md](03-console-csi-quadro.md) | Console com CSI / `quadro` |
| [04-rascunhos-recuperacao.md](04-rascunhos-recuperacao.md) | Rascunhos: fechar não apaga |
| [05-sair-confirma-todas-abas.md](05-sair-confirma-todas-abas.md) | Sair confirma todas as abas sujas |
| [06-abas-preservam-cursor.md](06-abas-preservam-cursor.md) | Abas guardam cursor, scroll e undo |

## P1 — comportamento de aula

| Arquivo | O quê |
|---|---|
| [07-layout-editor-repl.md](07-layout-editor-repl.md) | Editor com altura fill; REPL no meio |
| [08-pontos-de-parada-caminho.md](08-pontos-de-parada-caminho.md) | Pontos de parada: mesmo caminho do VM |
| [09-gutter-e-linha-atual.md](09-gutter-e-linha-atual.md) | Gutter, clique, linha atual |
| [10-snapshot-depuracao.md](10-snapshot-depuracao.md) | Aba só-leitura da fonte que rodou |
| [11-graficos-canvas-eventos.md](11-graficos-canvas-eventos.md) | Canvas, teclas, Destacar, F5 |
| [12-atalhos-nao-roubam-input.md](12-atalhos-nao-roubam-input.md) | Atalhos não roubam `leia`/REPL |
| [13-ciclo-de-vida-servidor.md](13-ciclo-de-vida-servidor.md) | Matar o servidor que a UI subiu |
| [14-dialogos-rpc-fora-da-ui.md](14-dialogos-rpc-fora-da-ui.md) | `rfd` e RPC fora da thread da UI |
| [15-ajuda-e-completar.md](15-ajuda-e-completar.md) | F1, `::`, completar, assinatura |

## P2 — polimento

| Arquivo | O quê |
|---|---|
| [16-titulo-e-menus.md](16-titulo-e-menus.md) | Título `*` e marcas nos menus |
| [17-projetos-recentes.md](17-projetos-recentes.md) | Menu de projetos recentes |
| [18-pares-e-substituir.md](18-pares-e-substituir.md) | `inicio`/`fim` e Ctrl+H |
| [19-arvore-do-projeto.md](19-arvore-do-projeto.md) | Árvore: expandir, refresh, menu |
| [20-barra-de-abas.md](20-barra-de-abas.md) | Barra de abas com scroll |
| [21-fonte-fallback.md](21-fonte-fallback.md) | Fonte monoespaçada com fallback |
| [22-erros-de-sintaxe-status.md](22-erros-de-sintaxe-status.md) | Erro de sintaxe sem inundar o status |
| [23-fila-stdout.md](23-fila-stdout.md) | Fila de stdout do F5 |
| [24-tree-sitter-ident.md](24-tree-sitter-ident.md) | `:ident` como no GTK |
| [25-windows-ime-empacote.md](25-windows-ime-empacote.md) | IME Windows e empacote; sem cutover |

Plano antigo de estágios: `docs/tarefas/freya-tasks.md`.
