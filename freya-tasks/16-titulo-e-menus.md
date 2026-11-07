# 16 — Título `*` e marcas nos menus

Prioridade: P2. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

A janela chama-se sempre `Expressa Aula`. A aba tem `●` mas o título
do SO não. Ver → Arquivos / Painel / Claro / Escuro não mostram estado
(checkmark). O GTK usa ações booleanas / `win.tema`.

## Correção

Título `Expressa Aula — nome.lep *` quando a aba atual está suja
(`Platform` / `WindowConfig` se o Freya deixar atualizar o título).
Nos menus Ver, marcar o item ativo (Arquivos, Painel, tema).

## Aceite

- Editar: o título ganha `*`; salvar tira.
- Ver → Escuro com o tema já escuro: o item aparece marcado; Claro
  não.
