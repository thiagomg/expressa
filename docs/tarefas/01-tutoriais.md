# Tutoriais na IDE

A primeira abertura do Expressa Aula mostra a janela **Bem-vindo ao Expressa**, com o texto para começar pelos tutoriais e o botão **Abrir tutoriais**. Esse botão abre `expressa-tutorial` e fecha a boas-vindas. **Começar** (ou fechar a janela) segue para a pasta do projeto. A tela só aparece uma vez (`boas_vindas=visto` em `~/.config/expressa-aula/prefs`).

Os tutoriais rodam no binário separado `expressa-tutorial` (crate `aula/tutorial`). O menu **Ajuda → Tutoriais** também o abre. A janela tem as abas Básico e Avançado, índice de capítulos à esquerda e o markdown renderizado à direita (`tutorial/basico.md` e `tutorial/avancado.md`). O texto vai embutido no executável; se existir a pasta `tutorial/` ao lado do binário ou em `/usr/share/expressa/tutorial`, essa cópia ganha.

Os instaladores passam a incluir o binário, o atalho e a pasta `tutorial/` (deb: `/usr/bin/expressa-tutorial` e `.desktop`; Windows: ao lado de `expressa-aula.exe` e no menu Iniciar).
