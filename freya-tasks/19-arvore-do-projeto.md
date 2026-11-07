# 19 — Árvore: expandir, refresh, menu

Prioridade: P2. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

A árvore tem botões Novo/Pasta/Nome/Apagar em baixo, sem menu de
contexto. Restaurar o último ficheiro não expande os pais nem
seleciona o nó. Salvar como fora da pasta visível não dá refresh.
Ficheiro novo no disco (outro programa) só aparece depois de reabrir
o projeto.

## Correção

- Clique direito (ou o menu que o Freya tiver): novo ficheiro, pasta,
  renomear, apagar — no nó clicado, como o GTK.
- Ao restaurar `prefs.arquivo`, abrir os `abertos` dos pais e marcar
  `selecionado`.
- Depois de criar/renomear/apagar/Salvar como dentro da raiz:
  `recarregar()`.
- Refresh manual barato: F5 da árvore não existe; um botão “…” já
  escolhe projeto — pode ganhar “Atualizar” se couber.

Nomes sem ponto continuam a ganhar `.lep`.

## Aceite

- Reiniciar com último ficheiro em `lib/tartaruga.lep`: `lib` vem
  expandido e o nó selecionado.
- Novo ficheiro na pasta selecionada aparece na lista e abre a aba.
- Apagar pede confirmação (já há `Dialogo::Apagar`).
