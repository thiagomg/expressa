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

Pacote `.deb` (Ubuntu 24.04 e 26.04, via Docker): `./empacotar-deb.sh`. Os arquivos saem em `dist/`. O pacote instala os dois editores, `expressa-aula` (GTK) e `expressa-aula-freya` (Freya), cada um com atalho no menu. Se a GPU falhar no Freya: `FREYA_RENDERER=software`. Vulkan não é obrigatório.

### Builds no GitHub

O workflow [Build packages](.github/workflows/build.yml) compila e empacota a cada
push na `main`, incluindo merges de pull requests. Ele não roda para outras
branches, pull requests ainda não integrados ou tags.

No GitHub, abra **Actions → Build packages → execução → Artifacts** para baixar:

| Artefato | Conteúdo |
| --- | --- |
| `expressa_*-1~ubuntu24.04_amd64.deb` | Pacote para Ubuntu 24.04 |
| `expressa_*-1~ubuntu26.04_amd64.deb` | Pacote para Ubuntu 26.04 |
| `ExpressaAula-*-Setup.exe` | Instalador Windows da Aula **GTK** (gvsbuild), runtime do Visual C++ |

Os três pacotes incluem o interpretador, visualizador de gráficos, IDE Aula GTK,
servidor e tutoriais. O editor Freya entra no `.deb` Linux; no Windows o zip
sem GTK sai de `empacotar-freya.ps1` na máquina nativa (`dist/expressa-aula-freya-windows.zip`).
O instalador GTK **não** é substituído. Os artefatos do GitHub ficam 30 dias.
No Windows, execute `ExpressaAula-*-Setup.exe` (GTK) ou o zip Freya; no Ubuntu,
`sudo apt install ./expressa_*.deb` (os dois atalhos no menu).

Os arquivos são produzidos em `dist/` nos runners e enviados diretamente, sem
um ZIP adicional, como artefatos da execução. Esse workflow não publica uma GitHub Release. O workflow separado
**Project storage** mantém seus próprios gatilhos de testes.

Especificação: [especificacao-linguagem.md](especificacao-linguagem.md).
