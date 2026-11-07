# 25 — IME Windows e empacote (sem cutover)

Prioridade: P2. O GTK `expressa-aula` **permanece** depois desta tarefa.
Não há cutover: o binário Freya não passa a chamar-se `expressa-aula`
e o crate `aula/ui` não se arquiva.

## Problema

O Stage 11 de `docs/tarefas/freya-tasks.md` ainda fala em “ship without
GTK” e em renomear no fim. A decisão atual: os dois editores seguem
no `.deb` e, no Windows, cada um no seu zip/instalador.

Falta o walkthrough nativo Windows (IME ABNT, diálogos, F5, exemplos,
`FREYA_RENDERER=software`). `empacotar-freya.ps1` já gera o zip sem
DLLs GTK. O Inno do GTK (`criar-instalador.ps1`) não é substituído.

## Correção

- Linux: o `.deb` continua a instalar `expressa-aula` **e**
  `expressa-aula-freya`. Documentar. Medir tamanhos depois do `strip`.
- Windows: walkthrough no hardware nativo — `função` / `não` (tarefa
  02 + 21), Abrir pasta, F5, `leia`, tema, exemplos em Documentos.
- Instalador Freya próprio se fizer falta; **não** tirar o instalador
  GTK.
- README: GPU má → `FREYA_RENDERER=software`. Sem Vulkan obrigatório.

## Aceite

- [ ] `.deb` numa máquina limpa: os dois atalhos no menu; F5 dum
  exemplo em cada binário.
- [ ] Windows: IME, F5, zip/instalador Freya **sem** DLLs GTK; o
  pacote GTK com gvsbuild continua a existir.
- [x] Cutover / arquivar GTK: **fora de âmbito**. Nunca faz parte
  desta lista.
