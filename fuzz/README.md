# Fuzz

Alvos para o [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz) (precisa do Rust nightly):

```text
cargo install cargo-fuzz
cargo +nightly fuzz run parse        # texto qualquer → parse()
cargo +nightly fuzz run executar     # texto qualquer → executa o programa
```

- `parse`: qualquer texto deve virar um `Program` ou um `ParseError`, sem pânico nem estouro de pilha.
- `executar`: qualquer programa deve terminar com resultado ou `RuntimeError`. Roda sem janelas (hosts nulos), sem entrada, com arquivos presos numa pasta temporária e 200 ms de limite.

Uma falha fica em `fuzz/artifacts/<alvo>/`. Para repetir:

```text
cargo +nightly fuzz run parse fuzz/artifacts/parse/crash-…
```

Hoje o `parse` estoura a pilha com parênteses aninhados demais. A correção está em `ajustes/02-quedas.md`.

Fora do CI de propósito: uns minutos de execução local depois de mexer no lexer, no parser ou no avaliador bastam.
