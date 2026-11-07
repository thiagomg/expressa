# 10 — Aba só-leitura da fonte que rodou

Prioridade: P1. O GTK `expressa-aula` permanece; este item só mexe em `aula/ui-freya`.

## Problema

`ir_para_pausa` abre o ficheiro do projeto e mexe o cursor a cada
pausa. O aluno perde o sítio onde estava a editar. A fonte que o VM
corre é a cópia em `exec-freya`; mudar a aba no meio **não** muda o
que já está a correr (isso está certo), mas a UI finge que sim.

O Stage 7 pedia uma aba snapshot só-leitura.

## Correção

No F6, abrir (ou reusar) uma aba só-leitura com o texto e o caminho
relativo da cópia. F10/F11/pausa movem o cursor **nessa** aba. A aba
do aluno fica onde estava. Fechar a snapshot no Fim / Parar / próximo
F5, ou deixá-la e atualizar no próximo F6.

`CodeEditor` tem `read_only(true)`.

Não misturar `dirty` da snapshot com rascunhos (tarefa 04).

## Aceite

- F6, editar a aba original noutra linha: o passo seguinte ainda
  destaca a snapshot; F5 seguinte corre o texto **atual** da aba
  (cópia nova).
- Clique num quadro da pilha abre a snapshot (ou o ficheiro do
  projeto, se a snapshot já fechou) naquela linha.
