# Expressa Aula

IDE integrada para ser usada por alunos

| Crate | Função |
|-------|--------|
| `expressa` (raiz do repositório) | Linguagem: analisador léxico, analisador sintático, executor |
| `expressa-aula-proto` | protobuf do gRPC |
| `expressa-aula-storage` | gravação atômica e revisões dos arquivos do projeto |
| `expressa-aula-server` | Repositório de arquivos: diretórios dos estudantes e local |
| `expressa-aula` (`aula/ui`) | editor de texto: lista, Executar (F5) / Depurar (F6) no próprio processo |

## Layout no disco

```text
--root /var/expressa-aula
  local/           # loopback / casa
  ana/
    bhaskara.lep
  bruno/
```

## Execução dos programas

O editor executa e depura os programas; o servidor só guarda arquivos. Em Executar/Depurar o editor cria uma pasta exclusiva em `execucoes/`, dentro dos dados locais de `expressa-aula` (`~/.local/share/expressa-aula/` no Linux, respeitando `$XDG_DATA_HOME`). Copia o projeto inteiro e sobrepõe os textos das abas abertas. A identidade usa o caminho canônico local ou o endereço normalizado do servidor, aluno e projeto; nomes de exibição iguais não compartilham a pasta. Cada execução faz uma cópia nova, sem reutilizar ou apagar a de outra execução.

Quando o programa termina, arquivos novos ou alterados (`salve_arquivo`, `salve_csv`, …) voltam ao projeto com a revisão original. Falhas ficam preservadas com a origem e o motivo, inclusive após outra execução ou reinício. Em projetos no servidor, **Arquivo → Sincronização…** abre uma janela com duas áreas: **Arquivos ainda não enviados ao servidor**, para arquivos que o programa gerou ou alterou, e **Edições não salvas**, para o texto das abas guardado neste computador. As listas mostram somente o projeto aberto. Você pode tentar salvar novamente, exportar uma cópia para outra pasta ou descartar as cópias após confirmação; as tentativas não substituem versões mais recentes. Registros danificados também podem ser exportados como cópias brutas.

Projetos locais não exibem o menu nem a janela de sincronização. As cópias de segurança continuam automáticas. Se um arquivo gerado não puder ser gravado na pasta do projeto, a IDE avisa e oferece tentar salvar novamente ou exportar. Ao reabrir o projeto, arquivos preservados por uma falha anterior também podem ser revisados.

Execuções sem pendências são removidas. Novas execuções são recusadas quando há pelo menos 100 execuções retidas ou 512 MiB de dados retidos verificáveis, até que o usuário gerencie os arquivos preservados. Esses limites controlam a admissão de novas execuções, não são uma cota de escrita para o programa em execução. Não há expiração nem remoção automática de arquivos ainda não salvos. Cópias antigas sem registro, em `~/.cache/expressa-aula/`, não são apagadas ou reenviadas automaticamente; confira essas pastas manualmente.

A execução não lê nem grava fora da pasta do projeto (`..`, caminhos absolutos). Não há limite de tempo: jogos e outros laços infinitos (`exemplos/jogos/conway_jogo_da_vida.lep`) continuam até **Parar**. O painel de saída guarda as últimas 5000 linhas; depois de `cls()`/`casa()` funciona como um terminal (cada linha substitui o quadro anterior).

## REPL

**Ver → REPL** abre o painel para experimentar expressões. As variáveis ficam disponíveis para os próximos comandos. A saída aparece enquanto o código executa; mensagens de erro aparecem em vermelho. **Parar** interrompe a avaliação atual, inclusive durante impressão contínua, e permite continuar usando a sessão. **Reiniciar** cancela a sessão anterior, abandona seus comandos na fila e esquece as variáveis. Fechar a IDE encerra também o REPL.

O painel conserva até 5000 linhas e 262144 caracteres, removendo o texto mais antigo. **Limpar** apaga apenas o texto exibido. A fila aceita até 16 avaliações, cada uma com até 64 KiB de código; se estiver cheia, o comando fica na entrada para tentar novamente. Resultados muito longos são abreviados com um aviso. `saia()` encerra a sessão do painel; **Reiniciar** permite começar outra. Se a sessão falhar ou perder a conexão com seu interpretador, o painel informa o problema e oferece o mesmo reinício.

## Operações e responsividade

Abrir, salvar, listar arquivos e analisar a sintaxe usam trabalho em segundo plano. **Operações**, na barra inferior, mostra o que está em andamento e permite cancelar leituras, conexões e a preparação de uma execução. A edição continua disponível enquanto você aguarda.

Salvar grava a versão enviada ao iniciar a operação. Se você continuar digitando, as novas alterações permanecem marcadas na aba e serão gravadas na próxima vez que salvar. Fechar a janela ou trocar de projeto aguarda a confirmação dos salvamentos; uma gravação antiga não autoriza descartar edições mais novas.

Leituras de texto aceitam até 8 MiB em projetos locais; o servidor aceita até 3 MiB por arquivo de texto. O servidor recusa gravações acima desse limite antes de substituir o arquivo existente. Listagens têm limite de 20000 entradas e 64 níveis de pastas.

## Subir o servidor

```bash
# servidor (se já não estiver no ar, o editor tenta iniciá-lo)
cargo run -p expressa-aula-server -- --root ./aula-data --bind 127.0.0.1:50051

# editor
cargo run -p expressa-aula
```

O instalador (Windows e `.deb`) copia os exemplos para `Documentos/Expressa Aula/exemplos`. Arquivos novos do pacote entram na pasta; o que o aluno já tiver ali não é substituído. O editor não copia de novo ao abrir. No Linux, `expressa --instalar-exemplos` faz a mesma cópia (o `postinst` chama isso para o usuário que instalou).

**Ajuda → Tutoriais** abre o binário `expressa-tutorial`. A Aula reabre a última pasta e o último arquivo. A barra de baixo mostra `linha:coluna`. **Ver → Ir para linha** (Ctrl+G) salta para um número.

Os comandos ficam no menu do topo (**Arquivo**, **Executar**, **Depurar**, **Ver**, **Ajuda**), cada um com o atalho à direita. **Arquivo → Sair** (Ctrl+Q) fecha o editor (com pergunta se houver alterações). F5 executa a aba atual ou **continua uma depuração pausada**, **F6 inicia a depuração**, F7 mostra/oculta arquivos, F8 o painel do depurador, F9 (ou clique no número da linha) ponto de parada, F10 próximo, F11 entrar, Shift+F11 sair, Ctrl+S salva. O caminho do projeto aparece à direita da barra de baixo. `mostre()` do módulo `cartesiano` abre o painel **Gráficos** (Destacar vira janela irmã e fica assim até Acoplar ou fechar). `mostre()` de `tela_grafica` abre uma janela irmã por `nova()`. Clique e tecla nessa janela voltam em `evento()`. O desenho ocupa o retângulo que o painel ou a janela lhe dão, como o `expressa-grafico`.

A linha **argumentos** vai para `argumentos[1]…` no programa. `escreva_erro` aparece em vermelho. **Parar** encerra Executar e Depurar. Clique na linha `erro: … em arquivo:linha` abre o arquivo na linha. Executar usa o texto de todas as abas abertas, salvas ou não.

**Abas**: cada arquivo abre na sua aba, com desfazer (Ctrl+Z / Ctrl+Shift+Z) e pontos de parada próprios; `●` marca alterações não salvas. Ctrl+N abre uma aba nova, Ctrl+W fecha. Fechar uma aba, trocar de projeto ou fechar a janela com alterações pergunta se quer salvar.

**Salvar e conflitos**: salvar grava uma cópia completa antes de substituir o arquivo.
Se outra pessoa ou programa alterou o arquivo desde a leitura, a aba continua com
suas alterações e oferece **Comparar**, **Recarregar**, **Salvar como…** ou **Substituir**.
Mesmo **Substituir** verifica novamente a versão comparada. Se o arquivo mudou
outra vez, aparece outro conflito. Recarregar descarta explicitamente as alterações
da aba; Cancelar as preserva. Um conflito durante Salvar ao fechar interrompe o
fechamento; depois de resolvê-lo, feche novamente.

Substituir no projeto e os arquivos gerados pela execução também verificam a versão
lida. Uma saída de programa em conflito não sobrescreve o projeto: texto abre em
uma aba nova com cópia de recuperação; os resultados também permanecem no registro
de arquivos ainda não salvos. Arquivos binários podem ser exportados pela janela
de sincronização, em projetos no servidor, ou pelo aviso de falha local. O transporte
de arquivos binários remotos continua pendente: arquivos binários gerados localmente
por uma execução de projeto remoto são preservados para exportação.

Atualize editor e servidor juntos. O servidor recusa atualizações sem revisão de
editores antigos; o editor novo recusa gravar em servidores sem o método de gravação
protegida, preservando a aba. A compilação da IDE e do servidor requer Rust 1.89 ou
mais recente.

**Proteção das edições**: a cada cinco segundos, o editor guarda uma cópia do texto alterado, inclusive de abas sem título e antes de escolher um projeto. Em projetos locais, ao reabrir após uma interrupção, a IDE oferece **Abrir texto em abas** para conferir e salvar. Em projetos no servidor, essas cópias ficam em **Arquivo → Sincronização… → Edições não salvas**, sem abrir perguntas automaticamente. Abrir uma cópia não envia nem substitui arquivos. Se o original não estiver disponível, o texto abre em uma nova aba, com o nome original na dica da aba. **Depois** ou fechar a pergunta mantém as cópias para outra tentativa, mesmo ao sair normalmente. Também é possível salvar cópias em outra pasta, sem substituir arquivos existentes ou apagar as cópias de segurança. Registros com formato inválido podem ser exportados no formato original.
Salvar ou descartar uma aba remove somente as cópias dessa aba. Ao escolher o
primeiro projeto, as abas que já estavam abertas continuam abertas para serem salvas.

**Edição**: Tab indenta como no Emacs (com linhas selecionadas, desloca o bloco; Shift+Tab volta), Ctrl+Alt+\\ reindenta a seleção, Enter já indenta a linha nova e `}`/`]`/`)`/`fim` voltam sozinhos. Ctrl+/ comenta ou descomenta, Ctrl+D duplica a linha, Alt+↑/↓ move. O par `inicio`/`fim` ou `{`/`}` do cursor fica destacado. **Ctrl+F** busca (Esc fecha). Clique direito na árvore (arquivo, pasta ou área vazia): novo arquivo/pasta (na pasta clicada), renomear, apagar. Delete também apaga o item selecionado.

**Ajuda da linguagem**: **Ajuda → Funções nativas** (ou **F1** fora de um nome) abre uma janela com todas as funções nativas, agrupadas por tema no núcleo (texto, coleção, …) e por `importe` nos módulos. A ficha de cada uma é a mesma de `ajuda("matriz::zeros")`. O filtro casa nome, tema e o resumo (`data` acha `crono::agora` e `formate_data`). **Ctrl+Espaço** mostra sugestões de funções nativas, do arquivo e dos `importe` (escolher `pinte` acrescenta `importe "tela"`); dentro de `importe "` sugere os módulos e os arquivos. **F1** no nome mostra a ficha; dentro de uma chamada também pode mostrar a assinatura e o argumento atual. Erros sob o cursor também aparecem na ajuda de F1. **Esc** fecha as sugestões ou a ficha. Não há balões automáticos ao digitar ou passar o mouse, evitando interrupções e trabalho de análise desnecessário em computadores antigos. **Ctrl+clique** ou **F12** vai para a definição (inclusive em outro arquivo e no próprio `importe`). Erros de sintaxe aparecem sublinhados enquanto digita, com a mensagem na barra de baixo.

**Sala de aula**: Ctrl+= / Ctrl+- (ou A+ / A−) aumentam e diminuem a letra, Ctrl+0 volta. **Ver → Claro** ou **Ver → Escuro** pinta a janela inteira (menus, painéis, editor e saída). A escolha fica em `~/.config/expressa-aula/prefs`.

Fechar o editor fecha as janelas irmãs (gráficos destacados e Ajuda) e encerra o `expressa-aula-server` que ele mesmo tiver iniciado. A janela abre maximizada; o código ocupa a maior parte da tela. Em janela estreita os painéis encolhem.

No depurador: a linha atual fica amarela na aba **Execução: arquivo**, uma cópia somente para leitura da fonte executada (inclusive módulos importados). Um erro durante a depuração para na linha em vermelho, com as variáveis visíveis, e Continuar encerra. **Observados** guarda nomes para ver o valor a cada passo; **Variáveis** lista o que está no escopo; **Pilha** as chamadas.

**Parar** fecha essa aba imediatamente e devolve o foco ao editor, enquanto o interpretador termina. Pausas que já estavam na fila não reabrem a visualização. Quando o programa termina normalmente ou encerra após um erro, a aba temporária também fecha. Se você já voltou a editar outra aba, ela continua selecionada.

Erros de **Executar (F5)** permanecem no console e destacam a linha na aba editável quando seu texto corresponde ao código executado. Se houve edições durante a execução, a IDE preserva o cursor e avisa que a posição pertence à versão anterior. **Ver → Ver código executado** abre essa versão somente quando solicitado; ela fica disponível até a próxima execução preparada ou a troca de projeto. Clicar em erros e na pilha após o término também navega para o editor, sem abrir automaticamente uma cópia somente leitura.

Cada execução usa uma fonte fixa e os pontos de parada definidos ao iniciá-la. Editar ou salvar um arquivo durante a execução não altera o programa em andamento: as mudanças, inclusive nos pontos de parada, valem na próxima execução. O editor mostra um aviso quando a fonte foi editada; **Ver fonte em execução** abre a cópia executada, e **Voltar ao editor** retorna à aba editável. F5/F6 na aba de execução inicia a próxima execução usando a versão atual do editor depois que a anterior terminar.

Os pontos de parada acompanham inserções e exclusões de linhas. Ao apagar o texto de um ponto, ele segue a próxima posição sobrevivente (ou o fim do arquivo); desfazer/refazer acompanha essa posição. Pontos que coincidem aparecem uma só vez. Um círculo cinza vazado indica uma linha sem instrução executável, ou uma fonte com erro de sintaxe; esse ponto fica inativo e é informado na saída ao depurar.


**Arquivo → Local → Abrir projeto…** escolhe a pasta do projeto neste computador; **Arquivo → Remoto → Conectar ao servidor…** conecta a um servidor (`aula-data/aluno`). O caminho (ou `url — aula-data/aluno`) fica à direita da barra de status.

**Local → Projetos recentes** mostra até dez pastas, com seus caminhos completos. **Remoto → Servidores anteriores** mostra até dez combinações de servidor e aluno; ao reconectar, escolha o projeto na lista (o último usado fica selecionado). No submenu **Remoto**, quando há um projeto remoto ativo, a lista de projetos identifica o servidor e o aluno e marca o projeto atual. Use **Novo projeto…** para criar outro projeto e **Sincronização…** para acompanhar os arquivos locais e remotos.

`leia("Seu nome:")` espera a resposta na própria saída: digite e aperte Enter, como num terminal (o interpretador avisa com `<<<EXPRESSA-LEIA>>>`). Na linha de comando:

```bash
expressa --marcador-leia arquivo.lep
```

Em casa: o servidor escuta no loopback, aluno `local`. Na sala: um servidor no computador do professor; os clientes GTK falam por RPC.

## Testes

```bash
cargo test -p expressa-aula -p expressa-aula-server -p expressa-aula-storage
cargo clippy -p expressa-aula -p expressa-aula-server -p expressa-aula-storage --all-targets -- -D warnings
```

Os testes GTK precisam de uma tela disponível e falham explicitamente quando não
conseguem inicializá-la. Em Linux sem sessão gráfica, com Xvfb instalado:

```bash
xvfb-run -a cargo test -p expressa-aula -p expressa-aula-server
```

Com `-- --nocapture`, a mensagem `GTK initialized: executing GUI test assertions`
confirma que o corpo dos testes GTK foi executado. Os testes do controlador de
documentos usam arquivos temporários e dispensam a janela completa da aplicação;
os testes de `execution_state` dispensam também o GTK.
