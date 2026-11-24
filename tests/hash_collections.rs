use expressa::runtime::{
    ExecutionBackend, ExecutionOptions, compile_exb, run_exb_with_options, run_with_options,
};

fn options(backend: ExecutionBackend) -> ExecutionOptions {
    ExecutionOptions {
        backend,
        ..Default::default()
    }
}
fn check(source: &str, expected: &str) {
    for backend in [ExecutionBackend::Ast, ExecutionBackend::Bytecode]
        .into_iter()
        .filter(|b| b.is_available())
    {
        let out = run_with_options(source, "hash.lep", "", options(backend))
            .unwrap_or_else(|e| panic!("{backend:?}: {e}"));
        assert_eq!(out.stdout, expected, "{backend:?}");
        assert_eq!(out.exit_code, 0);
    }
    let bytes = compile_exb(source, "hash.lep").unwrap();
    let out =
        run_exb_with_options(&bytes, "hash.lepc", "", options(ExecutionBackend::Bytecode)).unwrap();
    assert_eq!(out.stdout, expected, "EXB");
}

#[test]
fn maps_compare_by_keys_and_values_and_keep_key_types_distinct() {
    check(
        r#"
a = mapa([:b -> 2, :a -> 1])
b = mapa([:a -> 1, :b -> 2])
garanta(a == b)
a:a = 9
garanta(a != b e tamanho(a) == 2)
a[1] = "inteiro"
a[verdadeiro] = "bool"
a["1"] = "texto"
garanta(tamanho(a) == 5)
garanta(a[1] == "inteiro" e a[verdadeiro] == "bool" e a["1"] == "texto")
copia = remova(a, 1)
garanta(nao (copia contem 1) e a contem 1)
garanta(copia contem verdadeiro e copia contem "1")
garanta((mapa([:a -> 1, :a -> 2]) se_falhar { nada }) == nada)
garanta((remova(copia, 1) se_falhar { nada }) == nada)
"#,
        "",
    );
}

#[test]
fn rehashing_preserves_entries_aliases_and_cyclic_deep_copies() {
    check(
        r#"
m = mapa()
alias = m
para i em 0..2048 { m[i] = i * 3 }
para i em 0..2048 {
    garanta(m contem i)
    garanta(m[i] == i * 3)
    m[i] += 1
}
garanta(tamanho(m) == 2048 e alias[2047] == 6142)
shared = [7]
m:primeiro = shared
m:segundo = shared
m:self = m
c = clone(m)
garanta(c == m)
c:primeiro[0] = 9
garanta(c:segundo[0] == 9 e c:self:segundo[0] == 9)
garanta(m:primeiro[0] == 7)
para i em 0..2048 { garanta(c[i] == i * 3 + 1) }
// Independent cycles are equal even when their fields were inserted differently.
a = mapa([:x -> 1])
a:self = a
b = mapa()
b:self = b
b:x = 1
garanta(a == b)
b:x = 2
garanta(a != b)
"#,
        "",
    );
}

#[test]
fn sets_deduplicate_union_remove_and_iterate_without_an_order_requirement() {
    check(
        r#"
s = conjunto([:bia, :ana, :bia, 1, verdadeiro, "1"])
garanta(tamanho(s) == 5)
garanta(s == conjunto(["1", verdadeiro, :ana, 1, :bia]))
garanta(s contem 1 e s contem verdadeiro e s contem "1")
u = s + conjunto([:ana, :caio])
garanta(tamanho(u) == 6 e tamanho(s) == 5)
u += :caio
garanta(tamanho(u) == 6)
v = remova(u, verdadeiro)
garanta(nao (v contem verdadeiro) e v contem 1 e u contem verdadeiro)
garanta(clone(v) == v)
seen = mapa()
para x em v { seen[x] = verdadeiro }
garanta(tamanho(seen) == tamanho(v))
para x em v { garanta(seen contem x) }
garanta((remova(v, verdadeiro) se_falhar { nada }) == nada)
"#,
        "",
    );
}

#[test]
fn display_sorts_keys_and_elements_independently_of_insertion_and_hash_order() {
    check(
        r#"
escreva(mapa([2 -> "numero", falso -> "bool", :z -> "texto", :a -> nada]))
escreva(conjunto([:z, verdadeiro, 2, :a, falso, -1]))
escreva(mapa([:a -> nada, :z -> "texto", falso -> "bool", 2 -> "numero"]))
"#,
        "mapa{\"a\": nada, \"z\": \"texto\", 2: \"numero\", falso: \"bool\"}\nconjunto{\"a\", \"z\", -1, 2, falso, verdadeiro}\nmapa{\"a\": nada, \"z\": \"texto\", 2: \"numero\", falso: \"bool\"}\n",
    );
}
