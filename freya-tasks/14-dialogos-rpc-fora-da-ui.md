# 14 — Diálogos e RPC fora da thread da UI

Prioridade: P1. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

`rfd::FileDialog::pick_file` / `pick_folder` / `save_file` correm na
thread da UI. No Linux o `rfd` puxa portal/GTK e pode gelar o loop
winit (o mesmo género de freeze do `MainLoop` aninhado no GTK).

`Turma::esperar` faz `thread::spawn` + `join` de um `block_on` a
partir do clique: conectar espera até 2s, subir o servidor até ~2s
mais. A janela não pinta.

## Correção

Abrir diálogo e chamadas gRPC num `use_future` / canal. O clique só
dispara o pedido; o resultado aplica `abrir_caminho`, `abrir_raiz`,
lista de projetos, erro no status.

Não aninhar outro runtime Tokio: o `main` já entra num multi-thread.
`Handle::try_current().spawn` + mensagem de volta.

## Aceite

- Arquivo → Abrir…: a janela continua a aceitar Esc / mover enquanto
  o diálogo nativo está aberto (Wayland).
- Servidor local parado: Conectar mostra “Conectando…” e a UI não
  congela 2s; sucesso abre a lista de projetos, falha vai ao status.
- Abrir projeto numa pasta enorme recusada (`pasta_larga`) ainda
  devolve o recado, sem freeze.
