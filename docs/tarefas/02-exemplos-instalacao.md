# Exemplos só na instalação

A Aula deixou de copiar exemplos toda vez que abre (e ao reabrir a pasta de Documentos). Quem instala o pacote é quem faz a cópia.

Windows (`expressa-aula.iss`): na instalação e na atualização, `onlyifdoesntexist` leva para `Documentos\Expressa Aula\exemplos` os arquivos que ainda não estão lá. Edições do aluno permanecem.

Linux (`.deb`): os modelos ficam em `/usr/share/expressa/exemplos`. O `postinst` chama `expressa --instalar-exemplos` como o `SUDO_USER`, que copia só o que falta para `Documentos/Expressa Aula/exemplos`. Se a instalação foi como root sem `SUDO_USER`, o aluno (ou o professor) roda o mesmo comando uma vez.

`copy_missing` vive em `src/exemplos.rs`. Arquivo que já existe não é sobrescrito; arquivo novo do pacote entra ao lado.
