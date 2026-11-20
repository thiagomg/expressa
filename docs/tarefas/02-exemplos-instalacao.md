# Exemplos só na instalação

A Aula deixou de copiar exemplos toda vez que abre (e ao reabrir a pasta de Documentos). Quem instala o pacote é quem faz a cópia.

Windows (`expressa-aula.iss`): na instalação e na atualização, `onlyifdoesntexist` leva para `Documentos\Expressa\exemplos` os arquivos que ainda não estão lá. Edições do aluno permanecem.

Linux (`.deb`): os modelos ficam em `/usr/share/expressa/exemplos`. Na instalação e na atualização, o `postinst` chama `expressa --instalar-exemplos` com a identidade de cada usuário comum já existente (faixa `UID_MIN` a `UID_MAX` de `/etc/login.defs`, com diretório pessoal existente). Isso funciona também em instalações gráficas ou como root, sem depender de `SUDO_USER`. Falhas de cópia são exibidas na saída da instalação. Contas criadas depois da instalação podem rodar o mesmo comando uma vez.

O destino respeita `XDG_DOCUMENTS_DIR` e `user-dirs.dirs` em `XDG_CONFIG_HOME` (ou `~/.config`), inclusive caminhos com `$HOME`. Sem configuração, usa `~/Documentos`, ou `~/Documents` se essa pasta já existir. No Windows, o instalador usa a pasta Documentos do sistema (`{userdocs}`), inclusive quando redirecionada.

`copy_missing` vive em `src/exemplos.rs`. Arquivo que já existe não é sobrescrito; arquivo novo do pacote entra ao lado.
