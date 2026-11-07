# 13 — Ciclo de vida do servidor que a UI subiu

Prioridade: P1. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

`Turma::garantir_servidor_local` guarda o `Child` em `spawned` e nunca
o mata no fecho da janela (`turma.rs`). O GTK mata o filho que ele
próprio lançou; um servidor que o professor já tinha no ar fica.

`spawn_tutorial` usa `process_group(0)` (`ajuda.rs`). O GTK tirou isso
para o Ctrl+C no terminal matar o filho. O tutorial Freya sobrevive ao
SIGINT do pai.

## Correção

- No close da janela / Drop de `Turma`: se `spawned` for `Some`,
  SIGTERM e depois SIGKILL (Unix); `kill` no Windows. Não matar um
  servidor a que só se ligou.
- Tirar `process_group(0)` do tutorial.
- PID file opcional, no estilo GTK, se o Drop do Freya não correr em
  SIGINT — só se o teste manual mostrar órfão.

## Aceite

- Casa, servidor não estava no ar, Arquivo → Servidor… conecta,
  fecha a Aula Freya: a porta 50051 fica livre (o filho morreu).
- Servidor já no ar (professor): fechar a Aula Freya deixa o
  processo do professor.
- Ajuda → Tutoriais, Ctrl+C no terminal que lançou a Aula: o
  `expressa-tutorial` termina com o pai (Unix).
