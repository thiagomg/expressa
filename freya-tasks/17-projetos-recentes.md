# 17 — Menu de projetos recentes

Prioridade: P2. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

`Prefs` já guarda até 10 `recente=` (local e servidor). A UI Freya
nunca lista isso. Arquivo → Abrir projeto… só abre o diálogo de pasta.

## Correção

Submenu Arquivo → Recentes com os 10. Clique local chama `abrir_raiz`.
Clique servidor preenche URL/aluno/projeto e conecta (tarefa 14 para
não gelar). Entrada inválida some da lista e grava prefs.

## Aceite

- Abrir `Documentos/Expressa Aula/exemplos`, fechar, reabrir a Aula
  Freya: Recentes mostra essa pasta; clique reabre a árvore.
- Um projeto Turma usado aparece como `url — aluno/projeto`.
