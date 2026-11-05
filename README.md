# Expressa

Linguagem de programação em **português brasileiro**, para quem quer aprender a escrever código com uma passagem mais simples do que já se sabe da escola para a programação.

Arquivos `.lep`. Índices começam em **0**. Inspirada em SML, Rust, Lua, C, Pascal, D e algumas ideias próprias.

- **Um tipo número** — não há `i16`, `i32`, `i64`, `f32`, `f64`. Contas de caderno são: `0.1 + 0.2` é `0.3`.
- **Pouca ambiguidade** — uma leitura, um significado; o par de bloco combina (`{` com `}`, `inicio` com `fim`).
- **Baterias inclusas** — texto, listas, mapas, arquivo, tela, matemática, matriz e gráfico cartesiano, sem baixar biblioteca para o primeiro programa.
- **REPL com introspecção** — `ajuda`, `ajuda escreva`, `ls`, `ls mat`, `tipo`, `tipos`.
- **Editor com executar e depurar** — [expressa-aula](aula/README.md).
- **Manual e exemplos** — [tutorial](tutorial/basico.md), [avançado](tutorial/avancado.md), [manual](docs/manual.md), [folha](folha-de-consulta.md), [exemplos/](exemplos/).

Implementada em Rust.

```text
nome = leia("Qual o seu nome? ")
escreva("Olá, " + nome)

notas = [7, 8, 6.5]
soma = 0
para n em notas {
    soma += n
}
escreva("média: " + (soma / tamanho(notas)))
```

## Executando

O binário `expressa` (`expressa.exe` no Windows) no PATH:

```text
expressa                 REPL
expressa prog.lep        arquivo
expressa debug prog.lep  depurador
```

No REPL: `2 + 3`, `escreva("oi")`, `ajuda escreva`. `sair` ou Ctrl+D encerra.

Compilar: `cargo build --release` (Rust, edition 2024). Binários em `target/release/`.

Pacote `.deb` (Ubuntu 24.04 e 26.04, via Docker): `./empacotar-deb.sh`. Os arquivos saem em `dist/`.

### Builds no GitHub

O workflow [Build packages](.github/workflows/build.yml) compila e empacota a cada
push na `main`, incluindo merges de pull requests. Ele não roda para outras
branches, pull requests ainda não integrados ou tags.

No GitHub, abra **Actions → Build packages → execução → Artifacts** para baixar:

| Artefato | Conteúdo |
| --- | --- |
| `expressa-ubuntu-24.04-x64` | Pacote `.deb` para Ubuntu 24.04 |
| `expressa-ubuntu-26.04-x64` | Pacote `.deb` para Ubuntu 26.04 |
| `expressa-windows-x64` | ZIP portátil com executáveis, GTK e runtime do Visual C++ |

Os três pacotes incluem o interpretador, visualizador de gráficos, IDE Aula,
servidor e tutoriais. Os artefatos ficam disponíveis por **30 dias**. No Windows,
extraia o ZIP e abra `expressa-aula.exe` dentro da pasta `expressa-aula`; no Ubuntu,
instale o `.deb` correspondente com `sudo apt install ./expressa_*.deb`.

Os arquivos são produzidos em `dist/` nos runners e enviados como artefatos da
execução. Esse workflow não publica uma GitHub Release. O workflow separado
**Project storage** mantém seus próprios gatilhos de testes.

Especificação: [especificacao-linguagem.md](especificacao-linguagem.md).
