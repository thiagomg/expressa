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

Sem o `cargo-fuzz` instalado, o mesmo build à mão (o `--target` explícito deixa os build scripts sem instrumentação):

```text
cd fuzz
RUSTFLAGS="--cfg fuzzing -Cpasses=sancov-module \
  -Cllvm-args=-sanitizer-coverage-level=4 \
  -Cllvm-args=-sanitizer-coverage-inline-8bit-counters \
  -Cllvm-args=-sanitizer-coverage-pc-table \
  -Cllvm-args=-sanitizer-coverage-trace-compares \
  -Cdebug-assertions -Coverflow-checks" \
  cargo +nightly build --release --target x86_64-unknown-linux-gnu
mkdir -p corpus/parse && cp ../exemplos/*/*.lep corpus/parse/
target/x86_64-unknown-linux-gnu/release/parse corpus/parse -max_total_time=120
```

Última rodada: `parse` com 1,17 milhão de entradas em 2 minutos e `executar` com 244 mil programas em 2,5 minutos, sem nenhuma falha.

Fora do CI de propósito: uns minutos de execução local depois de mexer no lexer, no parser ou no avaliador bastam.
