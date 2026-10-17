# Expressa

Linguagem de programação em **português brasileiro**, para quem quer aprender a escrever código com uma passagem mais simples do que já se sabe da escola para a programação.

Arquivos `.lep`. Índices começam em **1**. Inspirada em SML, Rust, Lua, C, Pascal, D e algumas ideias próprias.

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

Especificação: [especificacao-linguagem.md](especificacao-linguagem.md).
